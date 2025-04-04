use std::path::Path;

use anyhow::{anyhow, Context, Result}; // TODO: Replace with real error, since this is a library
use borderless_kv_store::backend::lmdb::Lmdb;
use borderless_kv_store::Db;
use borderless_sdk::contract::Introduction;
use borderless_sdk::internal::registers::REGISTER_INPUT;
use borderless_sdk::{contract::CallAction, ContractId};
use vm::VmState;
use wasmtime::{Caller, Config, Engine, Instance, Linker, Module, Store};

mod vm;

pub struct Runtime<'a, S = Lmdb>
where
    S: Db,
{
    linker: Linker<VmState<'a, S>>,
    store: Store<VmState<'a, S>>,
    engine: Engine,
    instance: Option<Instance>,
}

impl<'a, S: Db> Runtime<'a, S> {
    pub fn new(storage: &'a S) -> Result<Self> {
        let db_ptr = storage.create_sub_db("contract-db")?;
        let state = VmState::new(storage, db_ptr);

        let mut config = Config::new();
        config.cranelift_opt_level(wasmtime::OptLevel::Speed);
        config.async_support(false);
        let engine = Engine::new(&config)?;
        // let module = Module::from_file(&engine, contract_path)?;

        let mut linker: Linker<VmState<S>> = Linker::new(&engine);

        // NOTE: We have to wrap the functions into a closure here, because they must be monomorphized
        // (as a generic function cannot be made into a function pointer)
        linker.func_wrap("env", "tic", |caller: Caller<'_, VmState<S>>| {
            vm::tic(caller)
        })?;
        linker.func_wrap("env", "toc", |caller: Caller<'_, VmState<S>>| {
            vm::toc(caller)
        })?;
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
        // TODO: Those two functions contain randomness, which is not good
        linker.func_wrap("env", "storage_random_key", vm::storage_random_key)?;
        linker.func_wrap("env", "rand", vm::rand)?;

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

        let store = Store::new(&engine, state);

        Ok(Self {
            linker,
            store,
            engine,
            instance: None,
        })
    }

    pub fn instantiate_contract(
        &mut self,
        contract_id: ContractId,
        path: impl AsRef<Path>,
    ) -> Result<()> {
        // TODO: We have to write a "store" that saves all modules
        let module = Module::from_file(&self.engine, path)?;

        self.store.data_mut().set_contract(contract_id);
        let instance = self.linker.instantiate(&mut self.store, &module)?;
        self.instance = Some(instance);
        Ok(())
    }

    pub fn process_transaction(&mut self, action: &CallAction) -> Result<()> {
        if let Some(instance) = self.instance {
            let run = instance.get_typed_func::<(), ()>(&mut self.store, "process_transaction")?;
            let action_bytes = action.to_bytes()?;
            self.store
                .data_mut()
                .set_register(REGISTER_INPUT, action_bytes);

            run.call(&mut self.store, ())?;
        } else {
            return Err(anyhow!("No contract is instantiated"));
        }
        Ok(())
    }

    pub fn process_introduction(&mut self, introduction: &Introduction) -> Result<()> {
        let instance = self.instance.context("No contract is instantiated")?;

        let run = instance.get_typed_func::<(), ()>(&mut self.store, "process_introduction")?;
        let bytes = introduction.to_bytes()?;
        self.store.data_mut().set_register(REGISTER_INPUT, bytes);

        run.call(&mut self.store, ())?;

        Ok(())
    }

    pub fn process_revocation(&mut self, revocation: bool) -> Result<()> {
        todo!()
    }

    pub fn read_action(&self, idx: usize) -> Result<Option<CallAction>> {
        self.store.data().read_action(idx)
    }
}
