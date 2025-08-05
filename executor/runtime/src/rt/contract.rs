use std::sync::Arc;
use std::time::Instant;

use ahash::HashMap;
use borderless::__private::registers::*;
use borderless::common::{Introduction, Revocation, Symbols};
use borderless::contracts::{BlockCtx, TxCtx};
use borderless::events::Events;
use borderless::{events::CallAction, ContractId};
use borderless::{BlockIdentifier, BorderlessId};
use borderless_kv_store::backend::lmdb::Lmdb;
use borderless_kv_store::Db;
use parking_lot::Mutex;
use wasmtime::{Caller, Config, Engine, ExternType, FuncType, Linker, Module};

use super::vm::{ActiveEntity, Commit};
use super::{
    code_store::CodeStore,
    vm::{self, VmState},
};
use crate::ACTION_TX_REL_SUB_DB;
use crate::{
    error::{ErrorKind, Result},
    CONTRACT_SUB_DB,
};
use crate::{log_shim::*, LEDGER_SUB_DB};

pub type SharedRuntime<S> = Arc<Mutex<Runtime<S>>>;

/*
 * Runtime TODO's:
 * - use one global engine for all runtimes <- per runtime type !
 * - make the Store a short lived object
 * - use per-runtime caching (as an Instance is bound to the Store)
 * - invalidate the cache, when re-creating the store
 * - check State::decode before introducing
 *
 */

pub struct Runtime<S = Lmdb>
where
    S: Db,
{
    linker: Linker<VmState<S>>,
    engine: Engine,
    contract_store: CodeStore<S>,
    mutability_lock: MutLock,
    block_ctx: Option<Vec<u8>>,
    executor: Option<Vec<u8>>,
}

impl<S: Db> Runtime<S> {
    pub fn new(storage: &S, contract_store: CodeStore<S>, lock: MutLock) -> Result<Self> {
        let start = Instant::now();
        // We create all necessary dub-databases, in case they don't exist
        let _ = storage.create_sub_db(CONTRACT_SUB_DB)?;
        let _ = storage.create_sub_db(ACTION_TX_REL_SUB_DB)?;
        let _ = storage.create_sub_db(LEDGER_SUB_DB)?;

        // Generate engine ( without async support )
        let mut config = Config::new();
        config.cranelift_opt_level(wasmtime::OptLevel::Speed);
        config.async_support(false);
        let engine = Engine::new(&config)?;

        let mut linker: Linker<VmState<S>> = Linker::new(&engine);

        // NOTE: We have to wrap the functions into a closure here, because they must be monomorphized
        // (as a generic function cannot be made into a function pointer)
        linker.func_wrap(
            "env",
            "print",
            |caller: Caller<'_, VmState<S>>, ptr, len, level| vm::print(caller, ptr, len, level),
        )?;
        // -- Register-API
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
        // -- Storage-API
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
            "storage_cursor",
            |caller: Caller<'_, VmState<S>>, base_key| vm::storage_cursor(caller, base_key),
        )?;
        linker.func_wrap("env", "storage_gen_sub_key", vm::storage_gen_sub_key)?;

        // -- Ledger-API
        linker.func_wrap(
            "env",
            "create_ledger_entry",
            |caller: Caller<'_, VmState<S>>, wasm_ptr, wasm_len| {
                vm::create_ledger_entry(caller, wasm_ptr, wasm_len)
            },
        )?;

        // NOTE: Those functions introduce side-effects;
        // they should only be used by us or during development of a contract
        linker.func_wrap("env", "tic", |caller: Caller<'_, VmState<S>>| {
            vm::tic(caller)
        })?;
        linker.func_wrap("env", "toc", |caller: Caller<'_, VmState<S>>| {
            vm::toc(caller)
        })?;
        linker.func_wrap("env", "rand", vm::rand)?;

        info!("Initialized runtime in: {:?}", start.elapsed());

        Ok(Self {
            linker,
            engine,
            contract_store,
            mutability_lock: lock,
            block_ctx: None,
            executor: None,
        })
    }

    pub fn into_shared(self) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(self))
    }

    /// Creates a new instance of the wasm module in our [`CodeStore`] for the given contract-id
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(%contract_id), err))]
    pub fn instantiate_contract(
        &mut self,
        contract_id: ContractId,
        module_bytes: &[u8],
    ) -> Result<()> {
        let module = Module::new(&self.engine, module_bytes)?;
        check_module(&self.engine, &module)?;
        self.contract_store.insert_contract(contract_id, module)?;
        Ok(())
    }

    /// Sets the currently active block
    ///
    /// This buffers the encoded [`BlockCtx`], to later write it to the dedicated register, so that the wasm side can query it.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(%block_id), err))]
    pub fn set_block(&mut self, block_id: BlockIdentifier, block_timestamp: u64) -> Result<()> {
        let ctx = BlockCtx {
            block_id,
            timestamp: block_timestamp,
        };
        self.block_ctx = Some(ctx.to_bytes()?);
        Ok(())
    }

    /// Sanity check for introductions
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, err))]
    pub fn check_module_and_state(
        &mut self,
        module_bytes: Vec<u8>,
        state: serde_json::Value,
    ) -> Result<(bool, Vec<String>)> {
        let module = Module::new(&self.engine, module_bytes)?;
        check_module(&self.engine, &module)?;
        let mut store = self.contract_store.create_store(&self.engine)?;
        let instance = self.linker.instantiate(&mut store, &module)?;

        // Prepare registers
        store
            .data_mut()
            .set_register(REGISTER_INPUT, state.to_string().into_bytes());

        // Call the actual function on the wasm side
        store.data_mut().prepare_exec(ActiveEntity::None)?;
        let success = match instance
            .get_typed_func::<(), ()>(&mut store, "parse_state")
            .and_then(|func| func.call(&mut store, ()))
        {
            Ok(()) => true,
            Err(_e) => false,
        };
        let log = store.data_mut().finish_exec(None)?;
        Ok((success, log.into_iter().map(|l| l.msg).collect()))
    }

    /// Sets the currently active executor
    ///
    /// This buffers the [`BorderlessId`] of the executor, to later write it into the dedicated register,
    /// so that the wasm side can query it.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(%executor_id), err))]
    pub fn set_executor(&mut self, executor_id: BorderlessId) -> Result<()> {
        let bytes = executor_id.into_bytes().to_vec();
        self.executor = Some(bytes);
        Ok(())
    }

    #[must_use = "You have to handle the output events of this function"]
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(contract_id = %cid, %writer), err))]
    pub fn process_transaction(
        &mut self,
        cid: &ContractId,
        action: CallAction,
        writer: &BorderlessId,
        tx_ctx: TxCtx,
    ) -> Result<Option<Events>> {
        let input = action.to_bytes()?;
        let events =
            self.process_chain_tx(*cid, input, *writer, tx_ctx, Some(Commit::Action(action)))?;
        Ok(events)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(contract_id = %introduction.id, %writer), err))]
    pub fn process_introduction(
        &mut self,
        introduction: Introduction,
        writer: &BorderlessId,
        tx_ctx: TxCtx,
    ) -> Result<()> {
        let cid = match introduction.id {
            borderless::prelude::Id::Contract { contract_id } => contract_id,
            borderless::prelude::Id::Agent { .. } => return Err(ErrorKind::InvalidIdType.into()),
        };
        // NOTE: The input for the introduction is not the introduction, but only the initial state!
        // The introduction itself is commited by the VmState
        let initial_state = introduction.initial_state.to_string().into_bytes();
        self.process_chain_tx(
            cid,
            initial_state,
            *writer,
            tx_ctx,
            Some(Commit::Introduction(introduction)),
        )?;
        Ok(())
    }

    // TODO: Calling process introduction on an already revoked contract should generate an error
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(contract_id = %revocation.id, %writer), err))]
    pub fn process_revocation(
        &mut self,
        revocation: Revocation,
        writer: &BorderlessId,
        tx_ctx: TxCtx,
    ) -> Result<()> {
        let input = revocation.to_bytes()?;
        let cid = match revocation.id {
            borderless::prelude::Id::Contract { contract_id } => contract_id,
            borderless::prelude::Id::Agent { .. } => return Err(ErrorKind::InvalidIdType.into()),
        };
        self.process_chain_tx(
            cid,
            input,
            *writer,
            tx_ctx,
            Some(Commit::Revocation(revocation)),
        )?;
        Ok(())
    }

    // TODO: Return Option<Events> to have None or use Events::default() ?
    /// Abstraction over all possible chain transactions
    ///
    /// In case of an error, the `VmState` is reset by this function.
    fn process_chain_tx(
        &mut self,
        cid: ContractId,
        input: Vec<u8>,
        writer: BorderlessId,
        tx_ctx: TxCtx,
        commit: Option<Commit>,
    ) -> Result<Option<Events>> {
        let tx_ctx_bytes = tx_ctx.to_bytes()?;
        let (instance, mut store) = self
            .contract_store
            .get_contract(&cid, &self.engine, &mut self.linker)?
            .ok_or_else(|| ErrorKind::MissingContract { cid })?;

        let mtx = self.mutability_lock.get_lock(&cid);
        let _guard = mtx.lock();

        let contract_method = match &commit {
            Some(Commit::Action(_)) => "process_transaction",
            Some(Commit::Introduction(_)) => "process_introduction",
            Some(Commit::Revocation(_)) => "process_revocation",
            Some(Commit::Other) => panic!("Commit::Other is reserved for actions"),
            None => "process_transaction", // NOTE: None is used for dry-runs of transactions
        };

        // Prepare registers
        store.data_mut().set_register(REGISTER_INPUT, input);
        store.data_mut().set_register(REGISTER_TX_CTX, tx_ctx_bytes);
        store
            .data_mut()
            .set_register(REGISTER_WRITER, writer.into_bytes().into());

        // Buffered registers
        store.data_mut().set_register(
            REGISTER_BLOCK_CTX,
            self.block_ctx.clone().unwrap_or_default(),
        );
        store
            .data_mut()
            .set_register(REGISTER_EXECUTOR, self.executor.clone().unwrap_or_default());

        // Call the actual function on the wasm side
        store
            .data_mut()
            .prepare_exec(ActiveEntity::contract_tx(cid, true, tx_ctx))?;
        let commit = match instance
            .get_typed_func::<(), ()>(&mut store, contract_method)
            .and_then(|func| func.call(&mut store, ()))
        {
            Ok(()) => {
                // We commit it the way that we are told to
                commit
            }
            Err(e) => {
                warn!("{contract_method} failed with error: {e}");
                // In this case we do not want to commit, so set it to `None`
                None
            }
        };
        let output = store.data().get_register(REGISTER_OUTPUT);
        let _log = store.data_mut().finish_exec(commit);

        // Return output events
        match output {
            Some(bytes) => Ok(Some(Events::from_bytes(&bytes)?)),
            None => Ok(None),
        }
    }

    /// Executes an action without commiting the state
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(contract_id = %cid, %writer), err))]
    pub fn perform_dry_run(
        &mut self,
        cid: &ContractId,
        action: &CallAction,
        writer: &BorderlessId,
    ) -> Result<()> {
        let input = action.to_bytes()?;
        let tx_ctx = TxCtx::dummy();
        let _out = self.process_chain_tx(*cid, input, *writer, tx_ctx, None)?;
        Ok(())
    }

    // --- NOTE: Maybe we should create a separate runtime for the HTTP handling ?

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(contract_id = %cid, %path), err))]
    pub fn http_get_state(&mut self, cid: &ContractId, path: String) -> Result<(u16, Vec<u8>)> {
        let (status, result) = self.process_http_call(cid, path, None, None, "http_get_state")?;
        Ok((status, result))
    }

    /// Uses a POST request to parse and generate a [`CallAction`] object.
    ///
    /// The return type is a nested result. The outer result type should convert to a server error,
    /// as it represents errors in the runtime itself.
    /// The inner error type comes from the wasm code and contains the error status and message.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(contract_id = %cid, %path), err))]
    pub fn http_post_action(
        &mut self,
        cid: &ContractId,
        path: String,
        payload: Vec<u8>,
        writer: &BorderlessId,
    ) -> Result<std::result::Result<CallAction, (u16, String)>> {
        let (status, result) =
            self.process_http_call(cid, path, Some(payload), Some(writer), "http_post_action")?;
        if status == 200 {
            let action =
                CallAction::from_bytes(&result).map_err(|_| ErrorKind::InvalidRegisterValue {
                    register: "http-result",
                    expected_type: "CallAction",
                })?;
            Ok(Ok(action))
        } else {
            let error = String::from_utf8(result).map_err(|_| ErrorKind::InvalidRegisterValue {
                register: "http-result",
                expected_type: "string",
            })?;
            Ok(Err((status, error)))
        }
    }

    fn process_http_call(
        &mut self,
        cid: &ContractId,
        path: String,
        payload: Option<Vec<u8>>,
        writer: Option<&BorderlessId>,
        http_method: &'static str,
    ) -> Result<(u16, Vec<u8>)> {
        let (instance, mut store) = self
            .contract_store
            .get_contract(cid, &self.engine, &mut self.linker)?
            .ok_or_else(|| ErrorKind::MissingContract { cid: *cid })?;

        // Set registers
        store
            .data_mut()
            .prepare_exec(ActiveEntity::contract_http(*cid))?;

        store
            .data_mut()
            .set_register(REGISTER_INPUT_HTTP_PATH, path.into_bytes());

        if let Some(payload) = payload {
            store
                .data_mut()
                .set_register(REGISTER_INPUT_HTTP_PAYLOAD, payload);
        }

        if let Some(writer) = writer {
            store
                .data_mut()
                .set_register(REGISTER_WRITER, writer.into_bytes().into());
        }

        // Buffered registers
        store.data_mut().set_register(
            REGISTER_BLOCK_CTX,
            self.block_ctx.clone().unwrap_or_default(),
        );
        store
            .data_mut()
            .set_register(REGISTER_EXECUTOR, self.executor.clone().unwrap_or_default());

        if let Err(e) = instance
            .get_typed_func::<(), ()>(&mut store, http_method)
            .and_then(|func| func.call(&mut store, ()))
        {
            error!("{http_method} failed with error: {e}");
        }
        // Get output
        let status = store.data().get_register(REGISTER_OUTPUT_HTTP_STATUS);

        let result = store.data().get_register(REGISTER_OUTPUT_HTTP_RESULT);

        // Finish the execution ( and commit nothing )
        let _log = store.data_mut().finish_exec(None)?;

        // Parse status
        let status = status.ok_or_else(|| ErrorKind::MissingRegisterValue("http-status"))?;
        let status_bytes = status
            .try_into()
            .map_err(|_| ErrorKind::InvalidRegisterValue {
                register: "http-status",
                expected_type: "u16",
            })?;
        let status = u16::from_be_bytes(status_bytes);

        let result = result.ok_or_else(|| ErrorKind::MissingRegisterValue("http-result"))?;
        Ok((status, result))
    }

    /// Returns the symbols of the contract
    pub fn get_symbols(&mut self, cid: &ContractId) -> Result<Option<Symbols>> {
        let (instance, mut store) = self
            .contract_store
            .get_contract(cid, &self.engine, &mut self.linker)?
            .ok_or_else(|| ErrorKind::MissingContract { cid: *cid })?;

        store.data_mut().prepare_exec(ActiveEntity::None)?;

        // In case the contract does not export any symbols, just return 'None'
        if let Err(e) = instance
            .get_typed_func::<(), ()>(&mut store, "get_symbols")
            .and_then(|func| func.call(&mut store, ()))
        {
            error!("get_symbols failed with error: {e}");
        }
        let output = store.data().get_register(REGISTER_OUTPUT);
        store.data_mut().finish_exec(None)?;

        let bytes = match output {
            Some(b) => b,
            None => return Ok(None),
        };
        let symbols = Symbols::from_bytes(&bytes)?;
        Ok(Some(symbols))
    }

    pub fn available_contracts(&self) -> Result<Vec<ContractId>> {
        self.contract_store.available_contracts()
    }

    /// Returns a copy of the underlying db handle
    pub fn get_db(&self) -> S {
        self.contract_store.get_db()
    }
}

type Lock = Arc<Mutex<()>>;

/// Global mutability lock for all contracts
///
/// Since we can only allow one mutable contract execution at a given time, we need a mechanism to ensure that.
/// The `MutLock` ensures this on a per-contract basis. It holds `RwLock`s for all agents and provides threadsafe access.
///
/// The logic is similar but not identical to rusts ownership rules. While there can be only one read-write (mutable) execution,
/// there can be multiple read-only (immutable) executions even if there is an ongoing read-write execution !
/// The reason behind this is basically that read-only executions do not produce storage operations that would change the state in the database.
/// In the `VmState`, all write operations are buffered until the execution is finished. If there would be two executions in parallel,
/// we might end up commiting changes to a state, that has already changed under the hood - which is not what we want.
/// However, if there is a writer thread, the readers do not care, and also the writer does not care about the readers.
/// The readers will use the old state, until the new one is commited by the runtime.
///
/// Note: In contrast to [`borderless_runtime::rt::agent::MutLock`], this version uses only synchronous lock primitives.
#[derive(Clone, Default)]
pub struct MutLock {
    map: Arc<Mutex<HashMap<ContractId, Lock>>>,
}

impl MutLock {
    /// Returns the `RwLock` for the given contract.
    ///
    /// If the contract-id is unknown, a new lock is created.
    pub fn get_lock(&self, cid: &ContractId) -> Lock {
        let mut map = self.map.lock();
        let lock = map.entry(*cid).or_default();
        lock.clone()
    }
}

fn check_module(engine: &Engine, module: &Module) -> Result<()> {
    let functions = [
        "process_transaction",
        "process_introduction",
        "process_revocation",
        "http_get_state",
        "http_post_action",
        "parse_state",
        "get_symbols",
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
  (export "parse_state" (func $placeholder))
  (export "get_symbols" (func $placeholder))
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
            "parse_state",
            "get_symbols",
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
