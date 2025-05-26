#![allow(unused_imports)]
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use borderless::__private::registers::*;
use borderless::agents::Init;
use borderless::contracts::{Introduction, Revocation, Symbols};
use borderless::events::Events;
use borderless::{events::CallAction, AgentId, BorderlessId};
use borderless_kv_store::backend::lmdb::Lmdb;
use borderless_kv_store::Db;
use log::{error, warn};
use tokio::sync::{mpsc, Mutex};
use wasmtime::{Caller, Config, Engine, ExternType, FuncType, Linker, Module, Store};

use super::{
    code_store::CodeStore,
    vm::{self, Commit, VmState},
};
use crate::db::logger::print_log_line;
use crate::{
    db::logger,
    error::{ErrorKind, Result},
    CONTRACT_SUB_DB,
};

pub mod tasks;

pub type SharedRuntime<S> = Arc<Mutex<Runtime<S>>>;

// NOTE: I think we have to use a different runtime for Contracts and SW-Agents;
//
// the linker should provide different host functions, and SW-Agents require async support, so everything will be async.
//
// There is however a big chunk, that is identical in both runtimes; so the question is, how to generalize over this.
// Also: In the real world, we may want to embed the process runtimes (aswell as HTTP-contract-runtimes ??) in a sort of
// executor pool, where we have a fixed list of wasm runtimes ready to execute the request.
// This executor pool should itself be threadsafe, so it can be shared among threads safely.

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
    pub fn new(storage: &S, contract_store: CodeStore<S>) -> Result<Self> {
        let db_ptr = storage.create_sub_db(CONTRACT_SUB_DB)?;
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

    // TODO: Define container type, how we want to bundle contracts etc. and use this here, instead of reading from disk.
    pub fn instantiate_sw_agent(
        &mut self,
        contract_id: AgentId,
        path: impl AsRef<Path>,
    ) -> Result<()> {
        let module = Module::from_file(&self.engine, path)?;
        check_module(&self.engine, &module)?;
        self.contract_store.insert_swagent(contract_id, module)?;
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
            .contract_store
            .get_agent(aid, &self.engine, &mut self.store, &mut self.linker)
            .await?
            .ok_or_else(|| ErrorKind::MissingAgent { aid: *aid })?;

        // Call the actual function on the wasm side
        let func = instance.get_typed_func::<(), ()>(&mut self.store, "on_init")?;
        self.store.data_mut().begin_agent_exec(*aid, false)?;

        if let Err(e) = func.call_async(&mut self.store, ()).await {
            warn!("initialize failed with error: {e}");
        }
        self.store.data_mut().finish_agent_exec(false)?;

        // Return output events
        let bytes = self
            .store
            .data()
            .get_register(REGISTER_OUTPUT)
            .ok_or_else(|| ErrorKind::MissingRegisterValue("init-output"))?;
        let init = Init::from_bytes(&bytes)?;

        // TODO: Maybe we register the websocket stuff in here ?

        Ok(init)
    }

    pub async fn process_ws_msg(&mut self, aid: &AgentId, msg: Vec<u8>) -> Result<Option<Events>> {
        self.call_mut(aid, msg, "on_ws_msg", Events::from_bytes)
            .await
    }

    pub async fn on_ws_open(&mut self, aid: &AgentId) -> Result<Option<Events>> {
        self.call_mut(aid, Vec::new(), "on_ws_open", Events::from_bytes)
            .await
    }

    pub async fn on_ws_error(&mut self, aid: &AgentId) -> Result<Option<Events>> {
        self.call_mut(aid, Vec::new(), "on_ws_error", Events::from_bytes)
            .await
    }

    pub async fn on_ws_close(&mut self, aid: &AgentId) -> Result<Option<Events>> {
        self.call_mut(aid, Vec::new(), "on_ws_close", Events::from_bytes)
            .await
    }

    pub async fn process_introduction(&mut self, introduction: Introduction) -> Result<Init> {
        // Parse action
        let input = introduction.to_bytes()?;
        let aid = match introduction.id {
            borderless::prelude::Id::Contract { .. } => return Err(ErrorKind::InvalidIdType.into()),
            borderless::prelude::Id::Agent { agent_id } => agent_id,
        };
        let init = self
            .call_mut(&aid, input, "process_introduction", Init::from_bytes)
            .await?;
        Ok(init.unwrap_or_default())
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
        self.call_mut(aid, input, "process_action", Events::from_bytes)
            .await
    }

    /// Helper function for mutable calls
    async fn call_mut<T, F, E>(
        &mut self,
        aid: &AgentId,
        input: Vec<u8>,
        method: &'static str,
        transformer: F,
    ) -> Result<Option<T>>
    where
        F: Fn(&[u8]) -> std::result::Result<T, E>,
        crate::error::Error: From<E>,
    {
        let instance = self
            .contract_store
            .get_agent(aid, &self.engine, &mut self.store, &mut self.linker)
            .await?
            .ok_or_else(|| ErrorKind::MissingAgent { aid: *aid })?;

        // Prepare registers
        self.store.data_mut().set_register(REGISTER_INPUT, input);

        // Call the actual function on the wasm side
        let func = instance.get_typed_func::<(), ()>(&mut self.store, method)?;
        self.store.data_mut().begin_agent_exec(*aid, true)?;

        let logs = match func.call_async(&mut self.store, ()).await {
            Ok(()) => self.store.data_mut().finish_agent_exec(true)?,
            Err(e) => {
                warn!("{method} failed with error: {e}");
                self.store.data_mut().finish_agent_exec(false)?
            }
        };
        // Just print the logs here
        logs.into_iter().for_each(print_log_line);

        // Return output events
        match self.store.data().get_register(REGISTER_OUTPUT) {
            Some(bytes) => Ok(Some(transformer(&bytes)?)),
            None => Ok(None),
        }
    }

    // --- NOTE: Maybe we should create a separate runtime for the HTTP handling ?

    pub async fn http_get_state(&mut self, aid: &AgentId, path: String) -> Result<(u16, Vec<u8>)> {
        // Get instance
        let instance = self
            .contract_store
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
        let log = self.store.data_mut().finish_agent_exec(false)?;

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
    ) -> Result<std::result::Result<CallAction, (u16, String)>> {
        let instance = self
            .contract_store
            .get_agent(aid, &self.engine, &mut self.store, &mut self.linker)
            .await?
            .ok_or_else(|| ErrorKind::MissingAgent { aid: *aid })?;

        // TODO: Can't I just convert this into a call-action on-spot ??
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

        // TODO: This function can modify the state
        // Call the function
        self.store.data_mut().begin_agent_exec(*aid, false)?;
        if let Err(e) = func.call_async(&mut self.store, ()).await {
            warn!("http_get_state failed with error: {e}");
        }
        // Finish the execution
        let log = self.store.data_mut().finish_agent_exec(false)?;

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
    pub async fn get_symbols(&mut self, aid: &AgentId) -> Result<Option<Symbols>> {
        let instance = self
            .contract_store
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
        self.store.data_mut().finish_agent_exec(false)?;

        let bytes = match self.store.data().get_register(REGISTER_OUTPUT) {
            Some(b) => b,
            None => return Ok(None),
        };
        let symbols = Symbols::from_bytes(&bytes)?;
        Ok(Some(symbols))
    }

    pub fn available_agents(&self) -> Result<Vec<AgentId>> {
        self.contract_store.available_swagents()
    }
}

// TODO: Agents have to export different functions
fn check_module(_engine: &Engine, _module: &Module) -> Result<()> {
    // let functions = [
    //     "process_transaction",
    //     "process_introduction",
    //     "process_revocation",
    //     "http_get_state",
    //     "http_post_action",
    // ];
    // for func in functions {
    //     let exp = module
    //         .get_export(func)
    //         .ok_or_else(|| ErrorKind::MissingExport { func })?;
    //     if let ExternType::Func(func_type) = exp {
    //         if !func_type.matches(&FuncType::new(engine, [], [])) {
    //             return Err(ErrorKind::InvalidFuncType { func }.into());
    //         }
    //     } else {
    //         return Err(ErrorKind::InvalidExport { func }.into());
    //     }
    // }
    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     const ALL_EXPORTS: &str = r#"
// (module
//   ;; Declare the function `placeholder`
//   (func $placeholder)

//   ;; Export the functions so they can be called from outside the module
//   (export "process_transaction" (func $placeholder))
//   (export "process_introduction" (func $placeholder))
//   (export "process_revocation" (func $placeholder))
//   (export "http_get_state" (func $placeholder))
//   (export "http_post_action" (func $placeholder))
// )
// "#;
//     fn remove_line_with_pattern(original: &str, pattern: &str) -> String {
//         // Create a new Vec to hold the processed lines
//         let mut new_lines = Vec::new();

//         for line in original.lines() {
//             // Check if the line contains the pattern
//             if !line.contains(pattern) {
//                 // Otherwise, push the original line
//                 new_lines.push(line);
//             }
//         }

//         // Collect the lines back into a single string
//         new_lines.join("\n")
//     }

//     #[test]
//     fn missing_exports() {
//         let mut config = Config::new();
//         config.cranelift_opt_level(wasmtime::OptLevel::Speed);
//         config.async_support(false);
//         let engine = Engine::new(&config).unwrap();

//         // These are the functions, that must not be missing
//         let functions = [
//             "process_transaction",
//             "process_introduction",
//             "process_revocation",
//             "http_get_state",
//             "http_post_action",
//         ];
//         for func in functions {
//             let wat_missing = remove_line_with_pattern(ALL_EXPORTS, func);
//             let module = Module::new(&engine, &wat_missing);
//             assert!(module.is_ok());
//             let err = check_module(&engine, &module.unwrap());
//             assert!(err.is_err());
//         }
//         let module = Module::new(&engine, &ALL_EXPORTS);
//         assert!(module.is_ok());

//         let err = check_module(&engine, &module.unwrap());
//         assert!(err.is_ok());
//     }
// }
