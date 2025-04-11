use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result}; // TODO: Replace with real error, since this is a library
use borderless_kv_store::backend::lmdb::Lmdb;
use borderless_kv_store::Db;
use borderless_sdk::__private::registers::*;
use borderless_sdk::contract::{BlockCtx, Introduction, TxCtx};
use borderless_sdk::http::{Request, Response};
use borderless_sdk::{
    contract::{ActionRecord, CallAction},
    ContractId,
};
use borderless_sdk::{BlockIdentifier, BorderlessId};
use vm::VmState;
use wasmtime::{Caller, Config, Engine, Instance, Linker, Module, Store};

pub mod logger;
mod vm;

const CONTRACT_SUB_DB: &str = "contract-db";

pub struct Runtime<S = Lmdb>
where
    S: Db,
{
    linker: Linker<VmState<S>>,
    store: Store<VmState<S>>,
    engine: Engine,
    contract_store: HashMap<ContractId, Instance>,
}

impl<S: Db> Runtime<S> {
    pub fn new(storage: &S) -> Result<Self> {
        let db_ptr = storage.create_sub_db(CONTRACT_SUB_DB)?;
        let start = Instant::now();
        let state = VmState::new(storage.clone(), db_ptr);

        let mut config = Config::new();
        config.cranelift_opt_level(wasmtime::OptLevel::Speed);
        config.async_support(false);
        let engine = Engine::new(&config)?;
        // let module = Module::from_file(&engine, contract_path)?;

        let mut linker: Linker<VmState<S>> = Linker::new(&engine);

        // NOTE: We have to wrap the functions into a closure here, because they must be monomorphized
        // (as a generic function cannot be made into a function pointer)
        linker.func_wrap(
            "env",
            "print",
            |caller: Caller<'_, VmState<S>>, ptr, len, level| vm::print(caller, ptr, len, level),
        )?;
        linker.func_wrap(
            "env",
            "read_register",
            |caller: Caller<'_, VmState<S>>, register_id, ptr| {
                vm::read_register(caller, register_id, ptr)
            },
        )?;
        linker.func_wrap(
            "env",
            "register_len",
            |caller: Caller<'_, VmState<S>>, register_id| vm::register_len(caller, register_id),
        )?;
        linker.func_wrap(
            "env",
            "write_register",
            |caller: Caller<'_, VmState<S>>, register_id, wasm_ptr, wasm_ptr_len| {
                vm::write_register(caller, register_id, wasm_ptr, wasm_ptr_len)
            },
        )?;
        linker.func_wrap(
            "env",
            "storage_read",
            |caller: Caller<'_, VmState<S>>, base_key, sub_key, register_id| {
                vm::storage_read(caller, base_key, sub_key, register_id)
            },
        )?;
        linker.func_wrap(
            "env",
            "storage_write",
            |caller: Caller<'_, VmState<S>>, base_key, sub_key, value_ptr, value_len| {
                vm::storage_write(caller, base_key, sub_key, value_ptr, value_len)
            },
        )?;
        linker.func_wrap(
            "env",
            "storage_remove",
            |caller: Caller<'_, VmState<S>>, base_key, sub_key| {
                vm::storage_remove(caller, base_key, sub_key)
            },
        )?;
        linker.func_wrap(
            "env",
            "storage_has_key",
            |caller: Caller<'_, VmState<S>>, base_key, sub_key| {
                vm::storage_has_key(caller, base_key, sub_key)
            },
        )?;

        linker.func_wrap(
            "env",
            "storage_begin_acid_txn",
            |caller: Caller<'_, VmState<S>>| vm::storage_begin_acid_txn(caller),
        )?;
        linker.func_wrap(
            "env",
            "storage_commit_acid_txn",
            |caller: Caller<'_, VmState<S>>| vm::storage_commit_acid_txn(caller),
        )?;

        // NOTE: Those functions introduce side-effects;
        // they should only be used by us or during development of a contract
        linker.func_wrap("env", "storage_gen_sub_key", vm::storage_gen_sub_key)?;
        linker.func_wrap("env", "timestamp", vm::timestamp)?;
        linker.func_wrap("env", "tic", |caller: Caller<'_, VmState<S>>| {
            vm::tic(caller)
        })?;
        linker.func_wrap("env", "toc", |caller: Caller<'_, VmState<S>>| {
            vm::toc(caller)
        })?;
        linker.func_wrap("env", "rand", vm::rand)?;

        let store = Store::new(&engine, state);

        log::info!("Initialized runtime in: {:?}", start.elapsed());

        Ok(Self {
            linker,
            store,
            engine,
            contract_store: HashMap::new(),
        })
    }

    pub fn instantiate_contract(
        &mut self,
        contract_id: ContractId,
        path: impl AsRef<Path>,
    ) -> Result<()> {
        // TODO: We have to write a "store" that saves all modules
        let module = Module::from_file(&self.engine, path)?;

        let instance = self.linker.instantiate(&mut self.store, &module)?;
        self.contract_store.insert(contract_id, instance);
        Ok(())
    }

    /// Sets the currently active block
    ///
    /// This write the [`BlockCtx`] to the dedicated register, so that the wasm side can query it.
    pub fn set_block(&mut self, block_id: BlockIdentifier, block_timestamp: u64) -> Result<()> {
        let ctx = BlockCtx {
            block_id,
            timestamp: block_timestamp,
        };
        self.store
            .data_mut()
            .set_register(REGISTER_BLOCK_CTX, ctx.to_bytes()?);
        Ok(())
    }

    // TODO: This is the interface of the executor
    // contract_id: ContractId,
    // writer_pid: ParticipantId,
    // tx_context: TxCtx, < TxIdentifier + block-timestamp
    // tx_type: TxType,
    // raw_data: Vec<u8>,
    // -> we also require the tx-sequence number

    pub fn process_transaction(
        &mut self,
        cid: &ContractId,
        action: &CallAction,
        writer: &BorderlessId,
        tx_ctx: TxCtx,
    ) -> Result<()> {
        let input = action.to_bytes()?;
        self.process_chain_tx("process_transaction", *cid, input, *writer, tx_ctx)
    }

    pub fn process_introduction(
        &mut self,
        introduction: &Introduction,
        writer: &BorderlessId,
        tx_ctx: TxCtx,
    ) -> Result<()> {
        let input = introduction.to_bytes()?;
        self.process_chain_tx(
            "process_introduction",
            introduction.contract_id,
            input,
            *writer,
            tx_ctx,
        )
    }

    pub fn process_revocation(
        &mut self,
        revocation: bool,
        _writer: &BorderlessId,
        _tx_ctx: TxCtx,
    ) -> Result<()> {
        todo!()
    }

    /// Abstraction over all possible chain transactions
    fn process_chain_tx(
        &mut self,
        contract_method: &str,
        cid: ContractId,
        input: Vec<u8>,
        writer: BorderlessId,
        tx_ctx: TxCtx,
    ) -> Result<()> {
        let instance = self
            .contract_store
            .get(&cid)
            .context("contract is not instantiated")?;

        self.store.data_mut().begin_mutable_exec(cid)?;

        // Prepare registers
        self.store.data_mut().set_register(REGISTER_INPUT, input);
        self.store
            .data_mut()
            .set_register(REGISTER_TX_CTX, tx_ctx.to_bytes()?);
        self.store
            .data_mut()
            .set_register(REGISTER_WRITER, writer.into_bytes().into());

        // Call the actual function on the wasm side
        let func = instance.get_typed_func::<(), ()>(&mut self.store, contract_method)?;
        func.call(&mut self.store, ())?;

        // Finish the execution
        self.store.data_mut().finish_mutable_exec()?;
        Ok(())
    }

    pub fn read_action(&self, cid: &ContractId, idx: usize) -> Result<Option<ActionRecord>> {
        self.store.data().read_action(cid, idx)
    }

    pub fn len_actions(&self, cid: &ContractId) -> Result<Option<u64>> {
        self.store.data().len_actions(cid)
    }

    // --- NOTE: Maybe we should create a separate runtime for the HTTP handling ?

    pub fn http_get_state(&mut self, cid: &ContractId, path: String) -> Result<Response> {
        let instance = self
            .contract_store
            .get(cid)
            .context("contract is not instantiated")?;
        self.store.data_mut().begin_immutable_exec(*cid)?;

        self.store
            .data_mut()
            .set_register(REGISTER_INPUT_HTTP_PATH, path.into_bytes());

        let func = instance.get_typed_func::<(), ()>(&mut self.store, "http_get_state")?;
        func.call(&mut self.store, ())?;

        let status = self
            .store
            .data()
            .get_register(REGISTER_OUTPUT_HTTP_STATUS)
            .context("missing http-status")?;
        let status = u16::from_be_bytes(status.try_into().unwrap());

        let result = self
            .store
            .data()
            .get_register(REGISTER_OUTPUT_HTTP_RESULT)
            .context("missing http-result")?;

        // Finish the execution
        self.store.data_mut().finish_immutable_exec()?;

        Ok(Response {
            status,
            payload: result,
        })
    }

    pub fn http_post_action(
        &mut self,
        _cid: &ContractId,
        _path: String,
        _payload: Vec<u8>,
    ) -> Result<Response> {
        todo!()
    }
}
