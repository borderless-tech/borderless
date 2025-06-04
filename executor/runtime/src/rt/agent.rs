use std::sync::Arc;
use std::time::Instant;

use ahash::HashMap;
use borderless::__private::registers::*;
use borderless::agents::Init;
use borderless::common::{Introduction, Symbols};
use borderless::events::Events;
use borderless::{events::CallAction, AgentId, BorderlessId};
use borderless_kv_store::backend::lmdb::Lmdb;
use borderless_kv_store::Db;
use log::{info, warn};
use parking_lot::Mutex as SyncMutex;
use tokio::sync::{mpsc, Mutex};
use wasmtime::{Caller, Config, Engine, ExternType, FuncType, Linker, Module, Store};

use super::vm::AgentCommit;
use super::{
    code_store::CodeStore,
    vm::{self, VmState},
};
use crate::db::logger::print_log_line;
use crate::{
    db::logger,
    error::{ErrorKind, Result},
    AGENT_SUB_DB,
};

pub mod tasks;

pub type SharedRuntime<S> = Arc<Mutex<Runtime<S>>>;

pub struct Runtime<S = Lmdb>
where
    S: Db,
{
    linker: Linker<VmState<S>>,
    store: Store<VmState<S>>,
    engine: Engine,
    agent_store: CodeStore<S>,
    mutability_lock: MutLock,
}

impl<S: Db> Runtime<S> {
    pub fn new(storage: &S, agent_store: CodeStore<S>, lock: MutLock) -> Result<Self> {
        let db_ptr = storage.create_sub_db(AGENT_SUB_DB)?;
        let start = Instant::now();
        let state = VmState::new_async(storage.clone(), db_ptr);

        let mut config = Config::new();
        config.cranelift_opt_level(wasmtime::OptLevel::Speed);
        config.async_support(true); // <- BIG difference
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
            "storage_cursor",
            |caller: Caller<'_, VmState<S>>, base_key| vm::storage_cursor(caller, base_key),
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

        // --- TODO: Playground for the new async api
        linker.func_wrap_async(
            "env",
            "send_http_rq",
            |caller: Caller<'_, VmState<S>>, (rq_head, rq_body, rs_head, rs_body, err)| {
                Box::new(vm::async_abi::send_http_rq(
                    caller, rq_head, rq_body, rs_head, rs_body, err,
                ))
            },
        )?;
        linker.func_wrap_async(
            "env",
            "send_ws_msg",
            |caller: Caller<'_, VmState<S>>, (msg_ptr, msg_len)| {
                Box::new(vm::async_abi::send_ws_msg(caller, msg_ptr, msg_len))
            },
        )?;

        linker.func_wrap("env", "timestamp", vm::timestamp)?;

        let store = Store::new(&engine, state);

        info!("Initialized runtime in: {:?}", start.elapsed());

        Ok(Self {
            linker,
            store,
            engine,
            agent_store,
            mutability_lock: lock,
        })
    }

    pub fn into_shared(self) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(self))
    }

    pub fn instantiate_sw_agent(
        &mut self,
        contract_id: AgentId,
        module_bytes: &[u8],
    ) -> Result<()> {
        let module = Module::new(&self.engine, module_bytes)?;
        check_module(&self.engine, &module)?;
        self.agent_store.insert_swagent(contract_id, module)?;
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

    /// Registers a new websocket client
    pub fn register_ws(&mut self, aid: AgentId) -> Result<mpsc::Receiver<Vec<u8>>> {
        let (tx, rx) = mpsc::channel(4);
        self.store.data_mut().register_ws(aid, tx)?;
        Ok(rx)
    }

    pub async fn initialize(&mut self, aid: &AgentId) -> Result<Init> {
        let instance = self
            .agent_store
            .get_agent(aid, &self.engine, &mut self.store, &mut self.linker)
            .await?
            .ok_or_else(|| ErrorKind::MissingAgent { aid: *aid })?;

        // Call the actual function on the wasm side
        let func = instance.get_typed_func::<(), ()>(&mut self.store, "on_init")?;
        self.store.data_mut().begin_agent_exec(*aid, false)?;

        if let Err(e) = func.call_async(&mut self.store, ()).await {
            warn!("initialize failed with error: {e}");
        }
        self.store.data_mut().finish_agent_exec(None)?;

        // Return output events
        let bytes = self
            .store
            .data()
            .get_register(REGISTER_OUTPUT)
            .ok_or_else(|| ErrorKind::MissingRegisterValue("init-output"))?;
        let init = Init::from_bytes(&bytes)?;

        Ok(init)
    }

    pub async fn process_ws_msg(&mut self, aid: &AgentId, msg: Vec<u8>) -> Result<Option<Events>> {
        self.call_mut(aid, msg, "on_ws_msg", AgentCommit::Other)
            .await
    }

    pub async fn on_ws_open(&mut self, aid: &AgentId) -> Result<Option<Events>> {
        self.call_mut(aid, Vec::new(), "on_ws_open", AgentCommit::Other)
            .await
    }

    pub async fn on_ws_error(&mut self, aid: &AgentId) -> Result<Option<Events>> {
        self.call_mut(aid, Vec::new(), "on_ws_error", AgentCommit::Other)
            .await
    }

    pub async fn on_ws_close(&mut self, aid: &AgentId) -> Result<Option<Events>> {
        self.call_mut(aid, Vec::new(), "on_ws_close", AgentCommit::Other)
            .await
    }

    pub async fn process_introduction(
        &mut self,
        introduction: Introduction,
    ) -> Result<Option<Events>> {
        let aid = match introduction.id {
            borderless::prelude::Id::Contract { .. } => return Err(ErrorKind::InvalidIdType.into()),
            borderless::prelude::Id::Agent { agent_id } => agent_id,
        };
        // NOTE: The input for the introduction is not the introduction, but only the initial state!
        // The introduction itself is commited by the VmState
        let initial_state = introduction.initial_state.to_string().into_bytes();
        self.call_mut(
            &aid,
            initial_state,
            "process_introduction",
            AgentCommit::Introduction { introduction },
        )
        .await
    }

    // OK; Just to get some stuff going; I want to just simply call an action, and execute an http-request with it.
    // That's more than enough to test stuff out.
    // TODO: Logging ?
    pub async fn process_action(
        &mut self,
        aid: &AgentId,
        action: CallAction,
    ) -> Result<Option<Events>> {
        // Parse action
        let input = action.to_bytes()?;
        self.call_mut(aid, input, "process_action", AgentCommit::Other)
            .await
    }

    /// Helper function for mutable calls
    async fn call_mut(
        &mut self,
        aid: &AgentId,
        input: Vec<u8>,
        method: &'static str,
        commit: AgentCommit,
    ) -> Result<Option<Events>> {
        let instance = self
            .agent_store
            .get_agent(aid, &self.engine, &mut self.store, &mut self.linker)
            .await?
            .ok_or_else(|| ErrorKind::MissingAgent { aid: *aid })?;

        let lock = self.mutability_lock.get_lock(aid);
        let _guard = lock.lock().await;

        // Prepare registers
        self.store.data_mut().set_register(REGISTER_INPUT, input);

        // Call the actual function on the wasm side
        let func = instance.get_typed_func::<(), ()>(&mut self.store, method)?;
        self.store.data_mut().begin_agent_exec(*aid, true)?;

        let logs = match func.call_async(&mut self.store, ()).await {
            Ok(()) => self.store.data_mut().finish_agent_exec(Some(commit))?,
            Err(e) => {
                warn!("{method} failed with error: {e}");
                self.store.data_mut().finish_agent_exec(None)?
            }
        };
        // Just print the logs here
        logs.into_iter().for_each(print_log_line);

        // Return output events
        match self.store.data().get_register(REGISTER_OUTPUT) {
            Some(bytes) => Ok(Some(Events::from_bytes(&bytes)?)),
            None => Ok(None),
        }
    }

    // --- NOTE: Maybe we should create a separate runtime for the HTTP handling ?

    pub async fn http_get_state(&mut self, aid: &AgentId, path: String) -> Result<(u16, Vec<u8>)> {
        // Get instance
        let instance = self
            .agent_store
            .get_agent(aid, &self.engine, &mut self.store, &mut self.linker)
            .await?
            .ok_or_else(|| ErrorKind::MissingAgent { aid: *aid })?;

        // Prepare registers
        self.store
            .data_mut()
            .set_register(REGISTER_INPUT_HTTP_PATH, path.into_bytes());

        // Get function
        let func = instance.get_typed_func::<(), ()>(&mut self.store, "http_get_state")?;

        // Call the function
        self.store.data_mut().begin_agent_exec(*aid, false)?;
        if let Err(e) = func.call_async(&mut self.store, ()).await {
            warn!("http_get_state failed with error: {e}");
        }
        // Finish the execution
        let log = self.store.data_mut().finish_agent_exec(None)?;

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

    // TODO: This will directly execute the action and return a list of events
    //
    // The question is, what should be returned via the web-api ?
    /// Uses a POST request to parse and generate a [`CallAction`] object.
    ///
    /// The return type is a nested result. The outer result type should convert to a server error,
    /// as it represents errors in the runtime itself.
    /// The inner error type comes from the wasm code and contains the error status and message.
    pub async fn http_post_action(
        &mut self,
        aid: &AgentId,
        path: String,
        payload: Vec<u8>,
        writer: &BorderlessId,
    ) -> Result<std::result::Result<(Events, CallAction), (u16, String)>> {
        let instance = self
            .agent_store
            .get_agent(aid, &self.engine, &mut self.store, &mut self.linker)
            .await?
            .ok_or_else(|| ErrorKind::MissingAgent { aid: *aid })?;

        let lock = self.mutability_lock.get_lock(aid);
        let _guard = lock.lock().await;

        // NOTE: We cannot convert the payload into a call-action on-spot, as we might call a nested route.
        // To be precise - we *could* do it here, but I think it is cleaner to leave this logic up to the wasm module,
        // as otherwise we may have to duplicate the logic here (and if it changes in the macro, we have to sync this with the code of the runtime etc.).
        self.store
            .data_mut()
            .set_register(REGISTER_INPUT_HTTP_PATH, path.into_bytes());

        self.store
            .data_mut()
            .set_register(REGISTER_INPUT_HTTP_PAYLOAD, payload);

        self.store
            .data_mut()
            .set_register(REGISTER_WRITER, writer.into_bytes().into());

        // Get function
        let func = instance.get_typed_func::<(), ()>(&mut self.store, "http_post_action")?;

        // Call the function
        self.store.data_mut().begin_agent_exec(*aid, true)?;
        if let Err(e) = func.call_async(&mut self.store, ()).await {
            warn!("http_get_state failed with error: {e}");
        }
        // Finish the execution
        let log = self
            .store
            .data_mut()
            .finish_agent_exec(Some(AgentCommit::Other))?;

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
            let events = match self.store.data().get_register(REGISTER_OUTPUT) {
                Some(b) => Events::from_bytes(&b)?,
                None => Events::default(),
            };
            let action = CallAction::from_bytes(&result)?;
            Ok(Ok((events, action)))
        } else {
            let error = String::from_utf8(result).map_err(|_| ErrorKind::InvalidRegisterValue {
                register: "http-result",
                expected_type: "string",
            })?;
            Ok(Err((status, error)))
        }
    }

    /// Returns the symbols of the contract
    pub async fn get_symbols(&mut self, aid: &AgentId) -> Result<Option<Symbols>> {
        let instance = self
            .agent_store
            .get_agent(aid, &self.engine, &mut self.store, &mut self.linker)
            .await?
            .ok_or_else(|| ErrorKind::MissingAgent { aid: *aid })?;

        // Get function
        let func = instance.get_typed_func::<(), ()>(&mut self.store, "get_symbols")?;

        // Call the function
        self.store.data_mut().begin_agent_exec(*aid, false)?;
        if let Err(e) = func.call_async(&mut self.store, ()).await {
            warn!("http_get_state failed with error: {e}");
        }
        // Finish the execution
        self.store.data_mut().finish_agent_exec(None)?;

        let bytes = match self.store.data().get_register(REGISTER_OUTPUT) {
            Some(b) => b,
            None => return Ok(None),
        };
        let symbols = Symbols::from_bytes(&bytes)?;
        Ok(Some(symbols))
    }

    pub fn available_agents(&self) -> Result<Vec<AgentId>> {
        self.agent_store.available_swagents()
    }
}

type Lock = Arc<Mutex<()>>;

/// Global mutability lock for all SW-Agents
///
/// Since we can only allow one mutable agent execution at a given time, we need a mechanism to ensure that.
/// The `MutLock` ensures this on a per-agent basis. It holds `RwLock`s for all agents and provides threadsafe access.
///
/// The logic is similar but not identical to rusts ownership rules. While there can be only one read-write (mutable) execution,
/// there can be multiple read-only (immutable) executions even if there is an ongoing read-write execution !
/// The reason behind this is basically that read-only executions do not produce storage operations that would change the state in the database.
/// In the `VmState`, all write operations are buffered until the execution is finished. If there would be two executions in parallel,
/// we might end up commiting changes to a state, that has already changed under the hood - which is not what we want.
/// However, if there is a writer thread, the readers do not care, and also the writer does not care about the readers.
/// The readers will use the old state, until the new one is commited by the runtime.
///
/// Note: In contrast to [`borderless_runtime::rt::contract::MutLock`],
/// this version uses asynchronous locks for the agents, and a synchronous lock only for the access of the hashmap.
#[derive(Clone, Default)]
pub struct MutLock {
    map: Arc<SyncMutex<HashMap<AgentId, Lock>>>,
}

impl MutLock {
    /// Returns the `RwLock` for the given agent.
    ///
    /// If the agent-id is unknown, a new lock is created.
    pub fn get_lock(&self, aid: &AgentId) -> Lock {
        let mut map = self.map.lock();
        let lock = map.entry(*aid).or_default();
        lock.clone()
    }
}

// NOTE: We could also check, if the websocket functions are exported,
// and do a consistency check, if the module uses a websocket.
// But maybe that's overkill.
fn check_module(engine: &Engine, module: &Module) -> Result<()> {
    let functions = [
        "on_init",
        "on_shutdown",
        "process_action",
        "process_introduction",
        "process_revocation",
        "http_get_state",
        "http_post_action",
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
  (export "on_init" (func $placeholder))
  (export "on_shutdown" (func $placeholder))
  (export "process_action" (func $placeholder))
  (export "process_introduction" (func $placeholder))
  (export "process_revocation" (func $placeholder))
  (export "http_get_state" (func $placeholder))
  (export "http_post_action" (func $placeholder))
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
            "on_init",
            "on_shutdown",
            "process_action",
            "process_introduction",
            "process_revocation",
            "http_get_state",
            "http_post_action",
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
