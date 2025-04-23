use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use action_log::ActionRecord;
use borderless::__private::registers::*;
use borderless::contracts::{BlockCtx, Introduction, Revocation, Symbols, TxCtx};
use borderless::events::Events;
use borderless::{events::CallAction, ContractId};
use borderless::{BlockIdentifier, BorderlessId};
use borderless_kv_store::backend::lmdb::Lmdb;
use borderless_kv_store::{Db, RawRead, RawWrite, Tx};
use error::ErrorKind;
use log::{error, warn};
use lru::LruCache;
use parking_lot::Mutex;
use vm::{Commit, VmState};
use wasmtime::{Caller, Config, Engine, ExternType, FuncType, Instance, Linker, Module, Store};

pub use error::{Error, Result};

pub mod action_log;
pub mod controller;
pub mod error;
pub mod http;
pub mod logger;
mod vm;

/// Sub-Database for all contract related data
const CONTRACT_SUB_DB: &str = "contract-db";

/// Sub-Database, where the wasm code is stored
const WASM_CODE_SUB_DB: &str = "wasm-code-db";

pub type SharedRuntime<S> = Arc<Mutex<Runtime<S>>>;

pub struct Runtime<S = Lmdb>
where
    S: Db,
{
    linker: Linker<VmState<S>>,
    store: Store<VmState<S>>,
    engine: Engine,
    contract_store: CodeStore<S>,
}

impl<S: Db> Runtime<S> {
    pub fn new(storage: &S, cache_size: NonZeroUsize) -> Result<Self> {
        let db_ptr = storage.create_sub_db(CONTRACT_SUB_DB)?;
        let start = Instant::now();
        let state = VmState::new(storage.clone(), db_ptr);

        let contract_store = CodeStore::new(storage.clone(), cache_size)?;

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

        // NOTE: Those functions introduce side-effects;
        // they should only be used by us or during development of a contract
        linker.func_wrap("env", "storage_gen_sub_key", vm::storage_gen_sub_key)?;
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
            contract_store,
        })
    }

    pub fn into_shared(self) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(self))
    }

    // TODO
    pub fn instantiate_contract(
        &mut self,
        contract_id: ContractId,
        path: impl AsRef<Path>,
    ) -> Result<()> {
        let module = Module::from_file(&self.engine, path)?;
        check_module(&self.engine, &module)?;
        self.contract_store.insert_contract(contract_id, module)?;
        Ok(())
    }

    /// Sets the currently active block
    ///
    /// This writes the [`BlockCtx`] to the dedicated register, so that the wasm side can query it.
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

    /// Sets the currently active executor
    ///
    /// This writes the [`BorderlessId`] of the executor to the dedicated register, so that the wasm side can query it.
    pub fn set_executor(&mut self, executor_id: BorderlessId) -> Result<()> {
        let bytes = executor_id.into_bytes().to_vec();
        self.store.data_mut().set_register(REGISTER_EXECUTOR, bytes);
        Ok(())
    }

    pub fn process_transaction(
        &mut self,
        cid: &ContractId,
        action: CallAction,
        writer: &BorderlessId,
        tx_ctx: TxCtx,
    ) -> Result<Option<Events>> {
        let input = action.to_bytes()?;

        self.store.data_mut().begin_mutable_exec(*cid)?;
        let events = self.process_chain_tx("process_transaction", *cid, input, *writer, &tx_ctx)?;
        self.store
            .data_mut()
            .finish_mutable_exec(Commit::Action { action, tx_ctx })?;
        Ok(events)
    }

    pub fn process_introduction(
        &mut self,
        introduction: Introduction,
        writer: &BorderlessId,
        tx_ctx: TxCtx,
    ) -> Result<()> {
        let input = introduction.to_bytes()?;
        self.store
            .data_mut()
            .begin_mutable_exec(introduction.contract_id)?;
        self.process_chain_tx(
            "process_introduction",
            introduction.contract_id,
            input,
            *writer,
            &tx_ctx,
        )?;
        self.store
            .data_mut()
            .finish_mutable_exec(Commit::Introduction {
                introduction,
                tx_ctx,
            })?;
        Ok(())
    }

    pub fn process_revocation(
        &mut self,
        revocation: Revocation,
        writer: &BorderlessId,
        tx_ctx: TxCtx,
    ) -> Result<()> {
        let input = revocation.to_bytes()?;

        self.store
            .data_mut()
            .begin_mutable_exec(revocation.contract_id)?;
        self.process_chain_tx(
            "process_revocation",
            revocation.contract_id,
            input,
            *writer,
            &tx_ctx,
        )?;
        self.store
            .data_mut()
            .finish_mutable_exec(Commit::Revocation { revocation, tx_ctx })?;
        Ok(())
    }

    // TODO: Return Option<Events> to have None or use Events::default() ?
    /// Abstraction over all possible chain transactions
    ///
    /// In case of an error, the `VmState` is reset by this function.
    fn process_chain_tx(
        &mut self,
        contract_method: &str,
        cid: ContractId,
        input: Vec<u8>,
        writer: BorderlessId,
        tx_ctx: &TxCtx,
    ) -> Result<Option<Events>> {
        let instance = self
            .contract_store
            .get_contract(&cid, &self.engine, &mut self.store, &mut self.linker)?
            .ok_or_else(|| ErrorKind::MissingContract { cid })?;

        // Prepare registers
        self.store.data_mut().set_register(REGISTER_INPUT, input);
        self.store
            .data_mut()
            .set_register(REGISTER_TX_CTX, tx_ctx.to_bytes()?);
        self.store
            .data_mut()
            .set_register(REGISTER_WRITER, writer.into_bytes().into());

        // Call the actual function on the wasm side
        if let Err(e) = instance
            .get_typed_func::<(), ()>(&mut self.store, contract_method)
            .and_then(|func| func.call(&mut self.store, ()))
        {
            warn!("{contract_method} failed with error: {e}");
            // NOTE: It is okay to abort the execution here with the finish_immutable_exec function,
            // because we only get here, if the wasm execution has failed. Therefore there are no
            // logs or actions to be commited to the database. We simply need this line to 'reset' the VmState for the next execution.
            self.store.data_mut().finish_immutable_exec()?;
        }

        // Return output events
        match self.store.data().get_register(REGISTER_OUTPUT) {
            Some(bytes) => Ok(Some(Events::from_bytes(&bytes)?)),
            None => Ok(None),
        }
    }

    /// Executes an action without commiting the state
    pub fn perform_dry_run(
        &mut self,
        cid: &ContractId,
        action: &CallAction,
        writer: &BorderlessId,
    ) -> Result<()> {
        let input = action.to_bytes()?;
        let tx_ctx = TxCtx::dummy().to_bytes()?;

        let instance = self
            .contract_store
            .get_contract(cid, &self.engine, &mut self.store, &mut self.linker)?
            .ok_or_else(|| ErrorKind::MissingContract { cid: *cid })?;

        self.store.data_mut().begin_immutable_exec(*cid)?;

        // Prepare registers
        self.store.data_mut().set_register(REGISTER_INPUT, input);
        self.store.data_mut().set_register(REGISTER_TX_CTX, tx_ctx);
        self.store
            .data_mut()
            .set_register(REGISTER_WRITER, writer.into_bytes().into());

        // Call the actual function on the wasm side
        if let Err(e) = instance
            .get_typed_func::<(), ()>(&mut self.store, "process_transaction")
            .and_then(|func| func.call(&mut self.store, ()))
        {
            warn!("dry-run of process_transaction failed with error: {e}");
        }
        // Finish the execution
        self.store.data_mut().finish_immutable_exec()?;
        Ok(())
    }

    // --- NOTE: Maybe we should create a separate runtime for the HTTP handling ?

    pub fn http_get_state(&mut self, cid: &ContractId, path: String) -> Result<(u16, Vec<u8>)> {
        let instance = self
            .contract_store
            .get_contract(cid, &self.engine, &mut self.store, &mut self.linker)?
            .ok_or_else(|| ErrorKind::MissingContract { cid: *cid })?;

        self.store.data_mut().begin_immutable_exec(*cid)?;

        self.store
            .data_mut()
            .set_register(REGISTER_INPUT_HTTP_PATH, path.into_bytes());

        if let Err(e) = instance
            .get_typed_func::<(), ()>(&mut self.store, "http_get_state")
            .and_then(|func| func.call(&mut self.store, ()))
        {
            warn!("http_get_state failed with error: {e}");
        }
        // Finish the execution
        let log = self.store.data_mut().finish_immutable_exec()?;

        let status = self
            .store
            .data()
            .get_register(REGISTER_OUTPUT_HTTP_STATUS)
            .ok_or_else(|| ErrorKind::MissingRegisterValue("http-status"))?;
        let status = u16::from_be_bytes(status.try_into().unwrap());

        let result = self
            .store
            .data()
            .get_register(REGISTER_OUTPUT_HTTP_RESULT)
            .ok_or_else(|| ErrorKind::MissingRegisterValue("http-result"))?;

        // Print the log
        for l in log {
            logger::print_log_line(l);
        }
        Ok((status, result))
    }

    /// Uses a POST request to parse and generate a [`CallAction`] object.
    ///
    /// The return type is a nested result. The outer result type should convert to a server error,
    /// as it represents errors in the runtime itself.
    /// The inner error type comes from the wasm code and contains the error status and message.
    pub fn http_post_action(
        &mut self,
        cid: &ContractId,
        path: String,
        payload: Vec<u8>,
        writer: &BorderlessId,
    ) -> Result<std::result::Result<CallAction, (u16, String)>> {
        let instance = self
            .contract_store
            .get_contract(cid, &self.engine, &mut self.store, &mut self.linker)?
            .ok_or_else(|| ErrorKind::MissingContract { cid: *cid })?;

        self.store.data_mut().begin_immutable_exec(*cid)?;

        self.store
            .data_mut()
            .set_register(REGISTER_INPUT_HTTP_PATH, path.into_bytes());

        self.store
            .data_mut()
            .set_register(REGISTER_INPUT_HTTP_PAYLOAD, payload);

        self.store
            .data_mut()
            .set_register(REGISTER_WRITER, writer.into_bytes().into());

        if let Err(e) = instance
            .get_typed_func::<(), ()>(&mut self.store, "http_post_action")
            .and_then(|func| func.call(&mut self.store, ()))
        {
            error!("http_post_action failed with error: {e}");
        }
        // Finish the execution
        let log = self.store.data_mut().finish_immutable_exec()?;

        let status = self
            .store
            .data()
            .get_register(REGISTER_OUTPUT_HTTP_STATUS)
            .ok_or_else(|| ErrorKind::MissingRegisterValue("http-status"))?;
        let status = u16::from_be_bytes(status.try_into().unwrap());

        let result = self
            .store
            .data()
            .get_register(REGISTER_OUTPUT_HTTP_RESULT)
            .ok_or_else(|| ErrorKind::MissingRegisterValue("http-result"))?;

        // Print the log
        for l in log {
            logger::print_log_line(l);
        }

        if status == 200 {
            let action = CallAction::from_bytes(&result)?;
            Ok(Ok(action))
        } else {
            let error = String::from_utf8(result).map_err(|_| ErrorKind::InvalidRegisterValue {
                register: "http-result",
                expected_type: "string",
            })?;
            Ok(Err((status, error)))
        }
    }

    /// Returns the symbols of the contract
    pub fn get_symbols(&mut self, cid: &ContractId) -> Result<Option<Symbols>> {
        let instance = self
            .contract_store
            .get_contract(cid, &self.engine, &mut self.store, &mut self.linker)?
            .ok_or_else(|| ErrorKind::MissingContract { cid: *cid })?;

        self.store.data_mut().begin_immutable_exec(*cid)?;

        // In case the contract does not export any symbols, just return 'None'
        if let Err(e) = instance
            .get_typed_func::<(), ()>(&mut self.store, "get_symbols")
            .and_then(|func| func.call(&mut self.store, ()))
        {
            error!("get_symbols failed with error: {e}");
        }
        self.store.data_mut().finish_immutable_exec()?;

        let bytes = match self.store.data().get_register(REGISTER_OUTPUT) {
            Some(b) => b,
            None => return Ok(None),
        };
        let symbols = Symbols::from_bytes(&bytes)?;
        Ok(Some(symbols))
    }

    pub fn read_action(&self, cid: &ContractId, idx: usize) -> Result<Option<ActionRecord>> {
        self.store.data().read_action(cid, idx)
    }

    pub fn len_actions(&self, cid: &ContractId) -> Result<Option<u64>> {
        self.store.data().len_actions(cid)
    }

    pub fn available_contracts(&self) -> Result<Vec<ContractId>> {
        self.contract_store.available_contracts()
    }
}

/// Storage for our webassembly code
struct CodeStore<S: Db> {
    db: S,
    db_ptr: S::Handle,
    cache: LruCache<ContractId, Instance, ahash::RandomState>,
}

impl<S: Db> CodeStore<S> {
    pub fn new(db: S, cache_size: NonZeroUsize) -> Result<Self> {
        let db_ptr = db.create_sub_db(WASM_CODE_SUB_DB)?;
        let cache = LruCache::with_hasher(cache_size, ahash::RandomState::default());
        Ok(Self { db, db_ptr, cache })
    }

    pub fn insert_contract(&self, cid: ContractId, module: Module) -> Result<()> {
        let module_bytes = module.serialize()?;
        let mut txn = self.db.begin_rw_txn()?;
        txn.write(&self.db_ptr, &cid, &module_bytes)?;
        txn.commit()?;
        Ok(())
    }

    pub fn get_contract(
        &mut self,
        cid: &ContractId,
        engine: &Engine,
        store: &mut Store<VmState<S>>,
        linker: &mut Linker<VmState<S>>,
    ) -> Result<Option<Instance>> {
        if let Some(instance) = self.cache.get(cid) {
            return Ok(Some(*instance));
        }
        let txn = self.db.begin_ro_txn()?;
        let module_bytes = txn.read(&self.db_ptr, cid)?;
        let module = match module_bytes {
            Some(bytes) => unsafe { Module::deserialize(engine, bytes)? },
            None => return Ok(None),
        };
        txn.commit()?;
        let instance = linker.instantiate(store, &module)?;
        self.cache.push(*cid, instance);
        Ok(Some(instance))
    }

    pub fn available_contracts(&self) -> Result<Vec<ContractId>> {
        use borderless_kv_store::*;

        let mut out = Vec::new();
        let txn = self.db.begin_ro_txn()?;
        let mut cursor = txn.ro_cursor(&self.db_ptr)?;
        // TODO: if we store contracts and agents in the same db, we have to change the logic here
        for (key, _value) in cursor.iter() {
            let cid = ContractId::from_bytes(
                key.try_into()
                    .map_err(|_| crate::Error::msg("failed to parse contract-id from storage"))?,
            );
            out.push(cid);
        }
        drop(cursor);
        txn.commit()?;
        Ok(out)
    }
}

fn check_module(engine: &Engine, module: &Module) -> Result<()> {
    let functions = [
        "process_transaction",
        "process_introduction",
        "process_revocation",
        "http_get_state",
        "http_post_action",
    ];
    for func in functions {
        let exp = module
            .get_export(func)
            .ok_or_else(|| ErrorKind::MissingExport { func })?;
        if let ExternType::Func(func_type) = exp {
            if !func_type.matches(&FuncType::new(engine, [], [])) {
                return Err(ErrorKind::InvalidFuncType { func }.into());
            }
        } else {
            return Err(ErrorKind::InvalidExport { func }.into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_EXPORTS: &str = r#"
(module
  ;; Declare the function `placeholder`
  (func $placeholder)

  ;; Export the functions so they can be called from outside the module
  (export "process_transaction" (func $placeholder))
  (export "process_introduction" (func $placeholder))
  (export "process_revocation" (func $placeholder))
  (export "http_get_state" (func $placeholder))
  (export "http_post_action" (func $placeholder))
)
"#;
    fn remove_line_with_pattern(original: &str, pattern: &str) -> String {
        // Create a new Vec to hold the processed lines
        let mut new_lines = Vec::new();

        for line in original.lines() {
            // Check if the line contains the pattern
            if !line.contains(pattern) {
                // Otherwise, push the original line
                new_lines.push(line);
            }
        }

        // Collect the lines back into a single string
        new_lines.join("\n")
    }

    #[test]
    fn missing_exports() {
        let mut config = Config::new();
        config.cranelift_opt_level(wasmtime::OptLevel::Speed);
        config.async_support(false);
        let engine = Engine::new(&config).unwrap();

        // These are the functions, that must not be missing
        let functions = [
            "process_transaction",
            "process_introduction",
            "process_revocation",
            "http_get_state",
            "http_post_action",
        ];
        for func in functions {
            let wat_missing = remove_line_with_pattern(ALL_EXPORTS, func);
            let module = Module::new(&engine, &wat_missing);
            assert!(module.is_ok());
            let err = check_module(&engine, &module.unwrap());
            assert!(err.is_err());
        }
        let module = Module::new(&engine, &ALL_EXPORTS);
        assert!(module.is_ok());

        let err = check_module(&engine, &module.unwrap());
        assert!(err.is_ok());
    }
}
