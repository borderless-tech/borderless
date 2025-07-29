//! WASM Virtual Machine
//!
//! This module contains the state of the virtual machine, that is shared across host function invocations,
//! and the concrete implementation of the ABI host functions, that are linked to the webassembly module by the runtime.

use borderless::__private::registers::*;
use borderless::common::Id;
use borderless::{
    __private::storage_keys::StorageKey,
    common::{Introduction, Revocation},
    contracts::TxCtx,
    events::CallAction,
    log::LogLine,
    AgentId, ContractId,
};
use borderless_kv_store::*;
use nohash::IntMap;
use rand::Rng;
use std::{
    cell::RefCell,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use wasmtime::{Caller, Extern, Memory};

#[cfg(feature = "agents")]
use tokio::sync::mpsc;

use crate::db::subscriptions::SubscriptionHandler;
use crate::{
    db::action_log::{ActionLog, ActionRecord},
    db::controller::{write_introduction, write_revocation},
    db::logger::Logger,
    error::ErrorKind,
    log_shim::*,
    Error, Result,
};

/// Virtual-Machine State
///
/// Represents the semi-persistent state of the wasm virtual machine.
/// This definition is identical for smart-contracts and software-agents,
/// with some additions for the async capabilities of the software-agents.
///
/// The main concepts behind this are the `registers`, which are used to share arbitrary data
/// between the host and wasm guest side. You can think of them as a shared hashmap, with functions on both sides for reading and writing
/// (e.g. [`read_register`] is called from guest code in wasm and [`VmState::get_register`] can be used from the host functions).
///
/// The `VmState` also tracks the currently active entity (contract or agent) and calculates the storage-keys based on its ID.
/// Every operation on the storage is buffered (see [`StorageOp`]) and commited to the database, if the execution was successful.
///
/// The `VmState` is also polymorph over the storage implementation.
pub struct VmState<S: Db> {
    registers: IntMap<u64, RefCell<Vec<u8>>>,
    db: S,
    db_ptr: S::Handle,

    last_timer: Option<Instant>,

    /// Current buffer of log output for the given contract
    log_buffer: Vec<LogLine>,

    /// Currently active contract or sw-agent
    active: ActiveEntity,

    _async: Option<AsyncState>,
}

impl<S: Db> VmState<S> {
    pub fn new(db: S, db_ptr: S::Handle) -> Self {
        VmState {
            registers: Default::default(),
            db,
            db_ptr,
            last_timer: None,
            log_buffer: Vec::new(),
            active: ActiveEntity::None,
            _async: None,
        }
    }

    pub fn new_async(db: S, db_ptr: S::Handle) -> Self {
        VmState {
            registers: Default::default(),
            db,
            db_ptr,
            last_timer: None,
            log_buffer: Vec::new(),
            active: ActiveEntity::None,
            _async: Some(AsyncState::default()),
        }
    }

    /// Marks the beginning of a new execution
    ///
    /// Sets the active entity and removes output artifacts from previous executions.
    /// Must be called before we can call `finish_exec`.
    ///
    /// # Errors
    ///
    /// Returns an error if there is still an active entity set.
    #[must_use = "You must handle the errors of this function"]
    pub fn prepare_exec(&mut self, active_entity: ActiveEntity) -> Result<()> {
        if self.active.is_some() {
            return Err(Error::msg(
                "Cannot start a new execution while something else is still active",
            ));
        }

        // Check output registers in debug mode
        debug_assert!(self.registers.remove(&REGISTER_OUTPUT).is_none());
        debug_assert!(self
            .registers
            .remove(&REGISTER_OUTPUT_HTTP_RESULT)
            .is_none());
        debug_assert!(self
            .registers
            .remove(&REGISTER_OUTPUT_HTTP_STATUS)
            .is_none());

        // Check log-buffer in debug mode
        debug_assert!(self.log_buffer.is_empty());

        // Set active entity
        self.active = active_entity;
        Ok(())
    }

    /// Marks the end of an execution
    ///
    /// Depending on the active entity and commit, the storage is either modified or not.
    /// For `ActiveEntity::None` or commit set to `None`, nothing is stored and instead the log-buffer is returned.
    ///
    /// Since the mutability is stored in the `ActiveEntity` (by either setting or not setting `db_txns`),
    /// you can set commit to `None`, if you want to "abort" a mutable execution (and not commit its state) in case of an error.
    ///
    /// Note: This function resets all output registers and the log-buffer,
    /// so be sure to copy the content from the output buffers *before* you call it.
    ///
    /// # Errors
    ///
    /// Calling this function while the `VmState` has a different active entity than
    /// specified in the commit will result in an error.
    pub fn finish_exec(&mut self, commit: Option<Commit>) -> Result<Vec<LogLine>> {
        // Take and reset log-output and active-entity
        let log_output = std::mem::take(&mut self.log_buffer);
        let active = std::mem::replace(&mut self.active, ActiveEntity::None);
        self.clear_cursor_registers()?;

        // Clear output registers, just in case
        self.registers.remove(&REGISTER_OUTPUT);
        self.registers.remove(&REGISTER_OUTPUT_HTTP_RESULT);
        self.registers.remove(&REGISTER_OUTPUT_HTTP_STATUS);

        // If we should not commit, we just return the log output.
        let commit = match commit {
            Some(c) => c,
            None => return Ok(log_output),
        };

        // Check active entity
        let (id, db_txns, tx_ctx) = match active {
            ActiveEntity::Contract {
                cid,
                db_txns,
                tx_ctx,
            } => (Id::contract(cid), db_txns, tx_ctx),
            ActiveEntity::Agent { aid, db_txns } => (Id::agent(aid), db_txns, None),
            ActiveEntity::None => return Ok(log_output),
        };

        // Start db-txn to commit log and state
        let now = Instant::now();
        let mut txn = self.db.begin_rw_txn()?;

        let logger = Logger::new(&self.db, id);
        logger.flush_lines(&log_output, &self.db_ptr, &mut txn)?;

        // If db_txns is none (immutable execution), we won't iterate here
        for op in db_txns.unwrap_or_default() {
            // Check, that all keys are user-keys - ignore system-keys.
            if !op.is_userspace() {
                warn!("Tried to write or remove a value with a storage-key that is not in user-space, id={id}");
                continue;
            }
            match op {
                StorageOp::Write { key, value } => txn.write(&self.db_ptr, &key, &value)?,
                StorageOp::Remove { key } => txn.delete(&self.db_ptr, &key)?,
            }
        }

        // Current timestamp ( milliseconds since epoch )
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("timestamp < 1970")
            .as_millis()
            .try_into()
            .expect("u64 should fit for 584942417 years");

        // Commit external item (introduction, action or revocation)
        match commit {
            Commit::Action(action) => {
                let cid = id.as_cid().expect("actions are only commited in contracts");
                let tx_ctx = tx_ctx.expect("actions are only commited in contracts");
                let action_log = ActionLog::new(&self.db, cid);
                action_log.commit(&self.db_ptr, &mut txn, &action, tx_ctx)?;
            }
            Commit::Introduction(mut introduction) => {
                assert_eq!(introduction.id, id);
                introduction.meta.active_since = timestamp;
                introduction.meta.tx_ctx_introduction = tx_ctx;
                write_introduction::<S>(&self.db_ptr, &mut txn, introduction.clone())?;
                SubscriptionHandler::new(&self.db).init(id, introduction.subscriptions)?;
            }
            Commit::Revocation(revocation) => {
                assert_eq!(revocation.id, id);
                write_revocation::<S>(&self.db_ptr, &mut txn, &revocation, timestamp, tx_ctx)?;
            }
            Commit::Other => { /* nothing to do */ }
        }

        // Commit txn
        txn.commit()?;
        let elapsed = now.elapsed();
        debug!("storage commit: {elapsed:?}");

        // Everything should be reset now
        debug_assert!(self.active.is_none());
        debug_assert!(self.log_buffer.is_empty());
        Ok(log_output)
    }

    /// Generates the storage key based on the currently active contract.
    ///
    /// Note: This function only generates user-keys, as values with system-keys must be commited by the host.
    fn get_storage_key(&self, base_key: u64, sub_key: u64) -> wasmtime::Result<StorageKey> {
        let key = self.active.storage_key(base_key, sub_key)?;
        Ok(key)
    }

    /// Writes the given value into the register.
    pub fn set_register(&mut self, register_id: u64, value: Vec<u8>) {
        self.registers.insert(register_id, value.into());
    }

    /// Returns a value from a register
    pub fn get_register(&self, register_id: u64) -> Option<Vec<u8>> {
        self.registers.get(&register_id).map(|v| v.borrow().clone())
    }

    /// Clears the registers from REGISTER_CURSOR until 2^64 -1
    fn clear_cursor_registers(&mut self) -> Result<()> {
        self.registers.retain(|&k, _| k < REGISTER_CURSOR);
        Ok(())
    }

    /// Removes a value from a register
    fn clear_register(&mut self, register_id: u64) {
        self.registers.remove(&register_id);
    }

    /// Tries to read the action with the given index for the currently active contract
    pub fn read_action(&self, cid: &ContractId, idx: usize) -> Result<Option<ActionRecord>> {
        ActionLog::new(&self.db, *cid).get(idx)
    }

    /// Returns the length of all actions
    pub fn len_actions(&self, cid: &ContractId) -> Result<u64> {
        ActionLog::new(&self.db, *cid).len()
    }

    /// Registers a websocket sender
    #[cfg(feature = "agents")]
    pub fn register_ws(&mut self, aid: AgentId, ch: mpsc::Sender<Vec<u8>>) -> Result<()> {
        let state = self._async.as_mut().ok_or_else(|| ErrorKind::NoAsync)?;
        state.clients.insert(aid, ch);
        Ok(())
    }
}

/// Parts of `VmState` that are only relevant for async execution
#[derive(Default)]
struct AsyncState {
    #[cfg(feature = "agents")]
    clients: ahash::HashMap<AgentId, mpsc::Sender<Vec<u8>>>,
}

/// Helper function to get the linear memory of the wasm module
fn get_memory(caller: &mut Caller<'_, VmState<impl Db>>) -> wasmtime::Result<Memory> {
    match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => Ok(mem),
        _ => Err(wasmtime::Error::msg("Failed to find memory")),
    }
}

/// Helper function to create a Vec<u8> that serves as a buffer with given length
///
/// # Safety
///
/// It is only meant to directly be written to, with the exact length given into this function.
/// Using the output vector in any other way may cause undefined behaviour !
fn create_buffer(len: u64) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(len as usize);
    #[allow(clippy::uninit_vec)]
    unsafe {
        buffer.set_len(len as usize);
    }
    buffer
}

/// Helper function to directly copy a bytes from the linear memory into a buffer
fn copy_wasm_memory(
    caller: &mut Caller<'_, VmState<impl Db>>,
    memory: &Memory,
    wasm_ptr: u64,
    wasm_ptr_len: u64,
) -> wasmtime::Result<Vec<u8>> {
    // Create buffer
    let mut buffer = create_buffer(wasm_ptr_len);

    // Read from memory
    memory
        .read(caller, wasm_ptr as usize, &mut buffer)
        .map_err(|e| wasmtime::Error::msg(format!("Failed to read from memory: {e}")))?;

    Ok(buffer)
}

// --- Begin to implement abi

/// Host function that starts a new timer.
///
/// This is the host implementation of `borderless_abi::tic` and must be linked by the runtime.
pub fn tic(mut caller: Caller<'_, VmState<impl Db>>) {
    caller.data_mut().last_timer = Some(Instant::now());
}

/// Host function that stops a new timer.
///
/// This is the host implementation of `borderless_abi::toc` and must be linked by the runtime.
pub fn toc(caller: Caller<'_, VmState<impl Db>>) -> wasmtime::Result<u64> {
    let timer = caller
        .data()
        .last_timer
        .ok_or_else(|| wasmtime::Error::msg("no timer present"))?;
    let elapsed = timer.elapsed();
    Ok(elapsed
        .as_nanos()
        .try_into()
        .expect("your program should not run for 584.942 years"))
}

// TODO: Change this to "log"
/// Host function that logs a string with a log-level.
///
/// This is the host implementation of `borderless_abi::print` and must be linked by the runtime.
pub fn print(
    mut caller: Caller<'_, VmState<impl Db>>,
    ptr: u64,
    len: u64,
    level: u32,
) -> wasmtime::Result<()> {
    // Get timestamp as early as possible
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();

    // Read string from WASM memory and print it
    // (Implementation details omitted for brevity)
    // let s = String::from_raw_parts(, length, capacity)
    let memory = get_memory(&mut caller)?;

    // Read memory
    let data = memory
        .data(&mut caller)
        .get(ptr as usize..(ptr + len) as usize)
        .ok_or_else(|| wasmtime::Error::msg("Memory access out of bounds"))?;

    // Construct message
    let msg =
        String::from_utf8(data.to_vec()).unwrap_or_else(|e| format!("Invalid UTF-8 sequence: {e}"));

    // Buffer log line
    let line = LogLine::new(timestamp, level, msg);
    caller.data_mut().log_buffer.push(line);

    Ok(())
}

/// Host function that reads bytes from some register.
///
/// Used to feed data from the host to the guest.
///
/// The guest (wasm side) must use [`register_len`] before reading,
/// to allocate enough space in the buffer behind the pointer `ptr`.
///
/// This is the host implementation of `borderless_abi::read_register` and must be linked by the runtime.
pub fn read_register(
    mut caller: Caller<'_, VmState<impl Db>>,
    register_id: u64,
    ptr: u64,
) -> wasmtime::Result<()> {
    // Get data
    //
    // Can we avoid the cloning here ?
    // -> Yes, by making this more complex.
    // Near defines a VmContext in nearcore/runtime/near-vm-runner/src/logic/logic.rs,
    // that holds a pointer to the memory and also holds the registers.
    // There are several indirections at work (e.g. memory is defined by a trait).
    //
    // We can explore this more in the future, for now I use a little hack,
    // by utilizing a RefCell container (so we can clone without cloning the inner Vec<u8>).
    let data = caller
        .data()
        .registers
        .get(&register_id)
        .cloned()
        .ok_or_else(|| wasmtime::Error::msg(format!("Register {register_id} not found")))?;

    // Get memory
    let memory = get_memory(&mut caller)?;

    // Check out-of-bounds read
    let mem_size = memory.data_size(&caller);
    if (ptr as usize) + data.borrow().len() > mem_size {
        return Err(wasmtime::Error::msg("Memory access out of bounds"));
    }

    // Write data from register to memory
    memory
        .write(&mut caller, ptr as usize, &data.borrow())
        .map_err(|e| wasmtime::Error::msg(format!("Failed to write to memory: {e}")))?;
    Ok(())
}

/// Host function that returns the number of bytes (length) that are stored in some register.
///
/// The guest (wasm side) must use [`register_len`] before calling [`read_register`],
/// to know, how much space must be allocated for reading.
///
/// This is the host implementation of `borderless_abi::register_len` and must be linked by the runtime.
pub fn register_len(caller: Caller<'_, VmState<impl Db>>, register_id: u64) -> u64 {
    match caller.data().registers.get(&register_id) {
        Some(data) => data.borrow().len() as u64,
        None => u64::MAX,
    }
}

/// Host function that writes a value to some register.
///
/// Used to feed data from the guest to the host.
///
/// This is the host implementation of `borderless_abi::write_register` and must be linked by the runtime.
pub fn write_register(
    mut caller: Caller<'_, VmState<impl Db>>,
    register_id: u64,
    wasm_ptr: u64,
    wasm_ptr_len: u64,
) -> wasmtime::Result<()> {
    // Get memory
    let memory = get_memory(&mut caller)?;

    // Copy value
    let value = copy_wasm_memory(&mut caller, &memory, wasm_ptr, wasm_ptr_len)?;

    // Set register
    caller.data_mut().set_register(register_id, value);
    Ok(())
}

// --- Storage api

/// Host function to write a value to the given storage location.
///
/// The storage location is defined by the `base_key` and `sub_key`, which are converted to a [`StorageKey`] by the `VmState` (see [`VmState::get_storage_key`]).
/// The write operation will be commited to the storage, after the execution of the contract or agent.
///
/// This is the host implementation of `borderless_abi::storage_write` and must be linked by the runtime.
pub fn storage_write(
    mut caller: Caller<'_, VmState<impl Db>>,
    base_key: u64,
    sub_key: u64,
    value_ptr: u64,
    value_len: u64,
) -> wasmtime::Result<()> {
    if caller.data().active.is_immutable() {
        return Ok(());
    }
    // Get memory
    let memory = get_memory(&mut caller)?;

    // Read value
    let value = copy_wasm_memory(&mut caller, &memory, value_ptr, value_len)?;

    // Build key
    let key = caller.data().get_storage_key(base_key, sub_key)?;

    // Push storage operation
    caller
        .data_mut()
        .active
        .push_storage(StorageOp::write(key, value))?;
    Ok(())
}

/// Host function to read a value from the given storage location.
///
/// The storage location is defined by the `base_key` and `sub_key`, which are converted to a [`StorageKey`] by the `VmState` (see [`VmState::get_storage_key`]).
///
/// This is the host implementation of `borderless_abi::storage_read` and must be linked by the runtime.
pub fn storage_read(
    mut caller: Caller<'_, VmState<impl Db>>,
    base_key: u64,
    sub_key: u64,
    register_id: u64,
) -> wasmtime::Result<()> {
    // Build key
    let key = caller.data().get_storage_key(base_key, sub_key)?;

    // Check, if there is an acid txn, and if so, commit the changes to that:
    let caller_data = &mut caller.data_mut();
    // If not, create a new transaction and instantly commit the changes
    let txn = caller_data.db.begin_ro_txn()?;
    let value = txn.read(&caller_data.db_ptr, &key)?.map(|v| v.to_vec());
    txn.commit()?;
    if let Some(value) = value {
        // Write to register
        caller.data_mut().set_register(register_id, value);
    } else {
        // NOTE: I think this should not be an error, as the storage_read abi
        // tries to read the register, and if the register has no value, if will be handled there.
        // So I think the cleanest way is to clear the register and return Ok(())
        caller_data.clear_register(register_id);
    }
    Ok(())
}

/// Host function to remove a value from the given storage location.
///
/// The storage location is defined by the `base_key` and `sub_key`, which are converted to a [`StorageKey`] by the `VmState` (see [`VmState::get_storage_key`]).
///
/// This is the host implementation of `borderless_abi::storage_remove` and must be linked by the runtime.
pub fn storage_remove(
    mut caller: Caller<'_, VmState<impl Db>>,
    base_key: u64,
    sub_key: u64,
) -> wasmtime::Result<()> {
    if caller.data().active.is_immutable() {
        return Ok(());
    }

    // Build key
    let key = caller.data().get_storage_key(base_key, sub_key)?;

    // Check, if there is an acid txn, and if so, commit the changes to that:
    let caller_data = &mut caller.data_mut();

    // Write changes to storage-buffer
    caller_data.active.push_storage(StorageOp::remove(key))?;
    Ok(())
}

/// Host function to create a storage cursor from the given base-key.
///
/// The storage locations are defined by a `base_key` and `sub_key`. The latter one is used for collections.
/// This function is used to query all available sub-keys for some base-key. The mechanism for that is a storage-cursor,
/// that starts iterating from (base-key, 0) until it hits the next base-key.
/// All sub-keys are then stored in the registers, so that the wasm-side can query them, while this function
/// returns the number of sub-keys found.
///
/// This is the host implementation of `borderless_abi::storage_cursor` and must be linked by the runtime.
pub fn storage_cursor(
    mut caller: Caller<'_, VmState<impl Db>>,
    base_key: u64,
) -> wasmtime::Result<u64> {
    // Build key (skips base_key)
    let key = caller.data().get_storage_key(base_key, 1)?;
    let tgt_prefix = key.get_prefix();

    // Set up DB access
    let db = &caller.data().db;
    let db_ptr = &caller.data().db_ptr;
    let txn = db.begin_ro_txn()?;

    // 1 - Move cursor at target key
    // 2 - Convert DB keys into StorageKey
    // 3 - Fetch all the keys matching the target prefix
    // 4 - For each resulting key, extract its sub-key
    let mut cursor = txn.ro_cursor(db_ptr)?;
    let keys: Vec<u64> = cursor
        .iter_from(&key)
        .map(|(key, _)| StorageKey::try_from(key).expect("Slice length error"))
        .take_while(|key| {
            let key_prefix = key.get_prefix();
            key_prefix.starts_with(&tgt_prefix)
        })
        .map(|key| key.sub_key())
        .collect();

    drop(cursor);
    drop(txn);

    let caller_data = &mut caller.data_mut();

    // Write keys into the registers
    for (i, key) in keys.iter().enumerate() {
        let bytes = key.to_le_bytes().to_vec();
        caller_data.set_register(REGISTER_CURSOR.saturating_add(i as u64), bytes);
    }
    // Return number of keys
    Ok(keys.len() as u64)
}

/// Host function to check, if a value exists at the given storage location.
///
/// The storage location is defined by the `base_key` and `sub_key`, which are converted to a [`StorageKey`] by the `VmState` (see [`VmState::get_storage_key`]).
///
/// This is the host implementation of `borderless_abi::storage_has_key` and must be linked by the runtime.
pub fn storage_has_key(
    mut caller: Caller<'_, VmState<impl Db>>,
    base_key: u64,
    sub_key: u64,
) -> wasmtime::Result<u64> {
    // Build key
    let key = caller.data().get_storage_key(base_key, sub_key)?;

    // Check, if there is an acid txn, and if so, commit the changes to that:
    let caller_data = &mut caller.data_mut();
    // If not, create a new transaction
    let txn = caller_data.db.begin_ro_txn()?;
    let result = txn.read(&caller_data.db_ptr, &key)?.is_some();
    txn.commit()?;
    Ok(result as u64)
}

/// Host function to generate a new random sub-key.
///
/// This is a very naive implementation, that relies on chance to not create a collision. Since our key-space is so stupidly large,
/// the chances are near zero to create the same storage key for the same base-key in the contract.
///
/// This is the host implementation of `borderless_abi::storage_gen_sub_key` and must be linked by the runtime.
pub fn storage_gen_sub_key() -> wasmtime::Result<u64> {
    let mut rng = rand::rng();
    Ok(rng.random())
}

/// Host function to generate a random number between `min` and `max`
///
/// Should only be used in tests or for software-agents, as randomness would introduce side-effects in the contracts.
///
/// This is the host implementation of `borderless_abi::rand` and must be linked by the runtime.
pub fn rand(min: u64, max: u64) -> wasmtime::Result<u64> {
    let mut rng = rand::rng();
    let value: u64 = rng.random_range(min..max);
    Ok(value)
}

/// Host function to returns the milliseconds since unix-epoch.
///
/// This is the host implementation of `borderless_abi::timestamp` and must be linked by the runtime.
#[cfg(feature = "agents")]
pub fn timestamp() -> wasmtime::Result<i64> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis()
        .try_into()
        .expect("i64 should fit for 292471208 years");
    Ok(timestamp)
}

/// Async (agent) ABI implementation
#[cfg(feature = "agents")]
pub mod async_abi {
    use borderless::Context;

    use super::*;

    use reqwest::{
        header::{HeaderMap, HeaderName, HeaderValue},
        Client, Method as ReqwestMethod, Request, Response,
    };
    use std::{result::Result, str::FromStr, time::Duration};

    /// Host function to send a websocket message
    ///
    /// The state of the websocket connection is not managed by the `VmState` itself.
    /// Instead, the `VmState` can communicate the request to send a new message over the open connection,
    /// via a channel (see [`tokio::mpsc::Sender`]). To avoid blocking - this function fails if the message
    /// was not picked up within a finite amount of time by the task with the ws-connection.
    ///
    /// This is the host implementation of `borderless_abi::send_ws_msg` and must be linked by the runtime.
    pub async fn send_ws_msg(
        mut caller: Caller<'_, VmState<impl Db>>,
        msg_ptr: u64,
        msg_len: u64,
    ) -> wasmtime::Result<u64> {
        let agent_id = caller
            .data()
            .active
            .is_agent()
            .ok_or_else(|| wasmtime::Error::msg("only sw-agents can send ws-msgs"))?;

        let memory = get_memory(&mut caller)?;

        // Read memory
        let data = memory
            .data(&mut caller)
            .get(msg_ptr as usize..(msg_ptr + msg_len) as usize)
            .ok_or_else(|| wasmtime::Error::msg("Memory access out of bounds"))?
            .to_vec();

        let state = caller
            .data()
            ._async
            .as_ref()
            .ok_or_else(|| wasmtime::Error::msg("missing async-state in async runtime"))?;

        match state.clients.get(&agent_id) {
            Some(ch) => {
                // We add a timeout here to not create a deadlock in case the channel is full
                match tokio::time::timeout(Duration::from_secs(10), ch.send(data)).await {
                    Ok(Ok(())) => return Ok(0),
                    Ok(Err(_)) => {
                        warn!("failed to send websocket message for agent {agent_id} - receiver closed");
                        Ok(1)
                    }
                    Err(_) => {
                        warn!("failed to send websocket message for agent {agent_id} - timeout");
                        Ok(2)
                    }
                }
            }
            None => {
                warn!("failed to send websocket message for agent {agent_id} - websocket is not registered");
                Ok(3)
            }
        }
    }

    /// Host function to send a http-request
    ///
    /// This is the host implementation of `borderless_abi::send_http_rq` and must be linked by the runtime.
    pub async fn send_http_rq(
        mut caller: Caller<'_, VmState<impl Db>>,
        register_rq_head: u64,
        register_rq_body: u64,
        register_rs_head: u64,
        register_rs_body: u64,
        register_failure: u64,
    ) -> wasmtime::Result<u64> {
        let head = caller
            .data_mut()
            .registers
            .remove(&register_rq_head)
            .context("missing rq-head")?
            .into_inner();

        let head = String::from_utf8(head)?;

        let body = caller
            .data_mut()
            .registers
            .remove(&register_rq_body)
            .context("missing rq-body")?
            .into_inner();

        // We do not use "?" to return the errors here, because these are client side errors, and not host related errors.
        //
        // We can use the `register_failure` to return the error message back to the caller on the wasm side.
        let client = Client::new();
        let rq = match parse_reqwest_request_from_parts(&client, &head, body) {
            Ok(rq) => rq,
            Err(e) => {
                caller
                    .data_mut()
                    .set_register(register_failure, e.into_bytes());
                return Ok(1);
            }
        };

        let rs = match client.execute(rq).await {
            Ok(rs) => rs,
            Err(e) => {
                caller
                    .data_mut()
                    .set_register(register_failure, e.to_string().into_bytes());
                return Ok(1);
            }
        };

        let (rs_head, rs_body) = match serialize_response_for_ffi(rs).await {
            Ok(rs) => rs,
            Err(e) => {
                caller
                    .data_mut()
                    .set_register(register_failure, e.to_string().into_bytes());
                return Ok(1);
            }
        };
        caller
            .data_mut()
            .set_register(register_rs_head, rs_head.into_bytes());
        caller.data_mut().set_register(register_rs_body, rs_body);

        Ok(0)
    }

    /// Helper function, that parses a [`reqwest::Request`] from raw parts
    ///
    /// As we designed the ABI, we communicate the header and the body seperately via registers.
    /// To actually have a typesafe interface, this function parses the header and body into a typed request.
    fn parse_reqwest_request_from_parts(
        client: &Client,
        head: &str,
        body: Vec<u8>,
    ) -> Result<Request, String> {
        let mut lines = head.lines();

        // Parse request line
        let request_line = lines
            .next()
            .ok_or_else(|| "Empty request head".to_string())?;
        let mut parts = request_line.split_whitespace();

        let method_str = parts
            .next()
            .ok_or_else(|| "No HTTP method found".to_string())?;
        let uri = parts.next().ok_or_else(|| "No URI found".to_string())?;
        let _version = parts
            .next()
            .ok_or_else(|| "No HTTP version found".to_string())?;
        // (We ignore HTTP version for reqwest, it manages it internally.)

        let method = ReqwestMethod::from_str(method_str).map_err(|e| e.to_string())?;

        // Parse headers
        let mut headers = HeaderMap::new();
        for line in lines {
            if line.trim().is_empty() {
                continue; // End of headers
            }
            if let Some((name, value)) = line.split_once(":") {
                let header_name = HeaderName::from_str(name.trim()).map_err(|e| e.to_string())?;
                let header_value =
                    HeaderValue::from_str(value.trim()).map_err(|e| e.to_string())?;
                headers.append(header_name, header_value);
            } else {
                return Err(format!("Malformed header line: {}", line));
            }
        }

        // Build the request
        let rq = {
            let client = client.request(method, uri).headers(headers);
            if !body.is_empty() {
                client.body(body)
            } else {
                client
            }
            .build()
            .map_err(|e| e.to_string())?
        };

        Ok(rq)
    }

    /// Helper function to serialize the response header
    ///
    /// See [`serialize_response_for_ffi`].
    fn serialize_response_head(resp: &Response) -> Result<String, String> {
        // Get status code and version
        let status = resp.status();
        let version = match resp.version() {
            reqwest::Version::HTTP_10 => "HTTP/1.0",
            reqwest::Version::HTTP_11 => "HTTP/1.1",
            reqwest::Version::HTTP_2 => "HTTP/2",
            other => return Err(format!("Unsupported HTTP version: {:?}", other)),
        };

        // Build the status line
        let mut head = format!(
            "{} {} {}\r\n",
            version,
            status.as_u16(),
            status.canonical_reason().unwrap_or("")
        );

        // Serialize headers
        for (name, value) in resp.headers().iter() {
            head.push_str(&format!(
                "{}: {}\r\n",
                name.as_str(),
                value
                    .to_str()
                    .map_err(|e| format!("failed to read header value: {e}"))?
            ));
        }

        head.push_str("\r\n"); // End of headers
        Ok(head)
    }

    /// Helper function to serialize the response back to wasm
    ///
    /// Basically the inverse of [`parse_reqwest_request_from_parts`].
    async fn serialize_response_for_ffi(resp: Response) -> Result<(String, Vec<u8>), String> {
        let head = serialize_response_head(&resp)?;
        // Get body as bytes
        let body = resp
            .bytes()
            .await
            .map_err(|e| format!("failed to read response body: {e}"))?
            .to_vec();

        Ok((head, body))
    }

    #[cfg(test)]
    mod async_abi_tests {
        use super::*;
        use reqwest::Method;

        #[test]
        fn test_valid_post_request() {
            let client = Client::new();
            let head = "POST https://example.com/api HTTP/1.1\r\nContent-Type: application/json\r\nX-Test: 42\r\n\r\n";
            let body = b"{\"hello\":\"world\"}".to_vec();

            let request = parse_reqwest_request_from_parts(&client, head, body.clone()).unwrap();

            assert_eq!(request.method(), Method::POST);
            assert_eq!(request.url().as_str(), "https://example.com/api");
            assert_eq!(request.headers()["Content-Type"], "application/json");
            assert_eq!(request.headers()["X-Test"], "42");
            assert_eq!(request.body().unwrap().as_bytes().unwrap(), body.as_slice());
        }

        #[test]
        fn test_valid_get_request() {
            let client = Client::new();
            let head = "GET https://example.com/ HTTP/1.1\r\nAccept: */*\r\n\r\n";
            let body = Vec::new();

            let request = parse_reqwest_request_from_parts(&client, head, body.clone()).unwrap();

            assert_eq!(request.method(), Method::GET);
            assert_eq!(request.url().as_str(), "https://example.com/");
            assert_eq!(request.headers()["Accept"], "*/*");
            assert!(request.body().is_none()); // No body for GET
        }

        #[test]
        fn test_missing_method_error() {
            let client = Client::new();
            let head = " https://example.com/api HTTP/1.1\r\nContent-Type: text/plain\r\n\r\n";
            let body = b"Missing method".to_vec();

            let result = parse_reqwest_request_from_parts(&client, head, body);
            assert!(result.is_err());
        }

        #[test]
        fn test_missing_uri_error() {
            let client = Client::new();
            let head = "POST HTTP/1.1\r\nContent-Type: text/plain\r\n\r\n";
            let body = b"Missing URI".to_vec();

            let result = parse_reqwest_request_from_parts(&client, head, body);
            assert!(result.is_err());
        }

        #[test]
        fn test_missing_version_error() {
            let client = Client::new();
            let head = "POST https://example.com/api\r\nContent-Type: text/plain\r\n\r\n";
            let body = b"Missing version".to_vec();

            let result = parse_reqwest_request_from_parts(&client, head, body);
            assert!(result.is_err());
        }

        #[test]
        fn test_malformed_header_error() {
            let client = Client::new();
            let head = "POST https://example.com/api HTTP/1.1\r\nBadHeaderWithoutColon\r\n\r\n";
            let body = b"Malformed header".to_vec();

            let result = parse_reqwest_request_from_parts(&client, head, body);
            assert!(result.is_err());
        }

        #[test]
        fn test_empty_head_error() {
            let client = Client::new();
            let head = "";
            let body = b"Empty head".to_vec();

            let result = parse_reqwest_request_from_parts(&client, head, body);
            assert!(result.is_err());
        }
    }
}

/// Represents a commit for some mutable execution
///
/// A commit instructs the `VmState` to actually save the state changes and the log-buffer.
/// Depending on what type of package was executed ( contract or agent ), there are some differences
/// in what is saved to disk. For example: Only contract actions write to the [`ActionLog`],
/// while introductions generally have their own behaviour, regardless of the package type.
pub enum Commit {
    /// commit a contract action
    Action(CallAction),
    /// commit a contract or agent introduction
    Introduction(Introduction),
    /// commit a contract or agent revocation
    Revocation(Revocation),
    /// commit an agent action, ws-msg or schedule
    Other,
}

/// Represents an executable entity in the VmState.
///
/// An entity can be executed with a mutable or immutable state.
/// Processing a chain-transaction on a contract requires a mutable state e.g.,
/// as this means the state of the contract can change and changes are written to the database.
///
/// An immutable execution is e.g. required for handling http-requests or performing dry-runs.
/// In such a case, calls to `storage_write` will be simply ignored.
pub enum ActiveEntity {
    Contract {
        cid: ContractId,
        // 'None', if immutable
        db_txns: Option<Vec<StorageOp>>,
        // 'None' for immutable http calls
        tx_ctx: Option<TxCtx>,
    },
    Agent {
        aid: AgentId,
        // 'None', if immutable
        db_txns: Option<Vec<StorageOp>>,
    },
    None,
}

impl ActiveEntity {
    pub fn contract_tx(cid: ContractId, mutable: bool, tx_ctx: TxCtx) -> Self {
        let db_txns = if mutable { Some(Vec::new()) } else { None };
        ActiveEntity::Contract {
            cid,
            db_txns,
            tx_ctx: Some(tx_ctx),
        }
    }

    pub fn contract_http(cid: ContractId) -> Self {
        ActiveEntity::Contract {
            cid,
            db_txns: None,
            tx_ctx: None,
        }
    }

    pub fn agent(aid: AgentId, mutable: bool) -> Self {
        let db_txns = if mutable { Some(Vec::new()) } else { None };
        ActiveEntity::Agent { aid, db_txns }
    }

    pub fn none() -> Self {
        ActiveEntity::None
    }

    fn is_some(&self) -> bool {
        !self.is_none()
    }

    fn is_none(&self) -> bool {
        matches!(self, ActiveEntity::None)
    }

    fn is_agent(&self) -> Option<AgentId> {
        match self {
            ActiveEntity::Agent { aid, .. } => Some(*aid),
            _ => None,
        }
    }

    /// Returns the storage key for the active entity
    fn storage_key(&self, base_key: u64, sub_key: u64) -> Result<StorageKey> {
        match self {
            ActiveEntity::Contract { cid, .. } => Ok(StorageKey::new(cid, base_key, sub_key)),
            ActiveEntity::Agent { aid, .. } => Ok(StorageKey::new(aid, base_key, sub_key)),
            ActiveEntity::None => Err(ErrorKind::NoActiveEntity.into()),
        }
    }

    /// Returns `true` if the active entity is immutable
    fn is_immutable(&self) -> bool {
        match self {
            ActiveEntity::Contract { db_txns, .. } | ActiveEntity::Agent { db_txns, .. } => {
                db_txns.is_none()
            }
            ActiveEntity::None => false,
        }
    }

    /// Pushes a storage operation to the storage buffer - if any
    ///
    /// Returns an error if there is either no active entity
    /// or if the active entity is immutable.
    fn push_storage(&mut self, op: StorageOp) -> Result<()> {
        match self {
            ActiveEntity::Contract { db_txns, .. } | ActiveEntity::Agent { db_txns, .. } => {
                if let Some(db_txns) = db_txns {
                    db_txns.push(op);
                    Ok(())
                } else {
                    Err(ErrorKind::Immutable.into())
                }
            }
            ActiveEntity::None => Err(ErrorKind::NoActiveEntity.into()),
        }
    }
}

/// Enum that represents a storage operation
///
/// All storage operations are commited to the key-value-store
/// once the contract finished its execution.
pub enum StorageOp {
    /// Writes the given value to the storage key
    Write { key: StorageKey, value: Vec<u8> },
    /// Removes the value at the given storage key
    Remove { key: StorageKey },
}

impl StorageOp {
    pub fn write(key: StorageKey, value: Vec<u8>) -> Self {
        Self::Write { key, value }
    }

    pub fn remove(key: StorageKey) -> Self {
        Self::Remove { key }
    }

    pub fn is_userspace(&self) -> bool {
        match self {
            StorageOp::Write { key, .. } | StorageOp::Remove { key } => key.is_user_key(),
        }
    }
}

#[cfg(test)]
mod tests {
    use backend::lmdb::Lmdb;
    use tempfile::{tempdir, TempDir};

    use super::*;

    fn dummy_vm_state() -> (VmState<Lmdb>, TempDir) {
        let tmp_dir = tempdir().expect("failed to create tmp directory for testing");
        let db = Lmdb::new(tmp_dir.path(), 2).expect("failed to create lmdb");
        let db_ptr = db
            .create_sub_db("dummy-sub-db")
            .expect("failed to create sub-db");
        let state = VmState::new(db, db_ptr);
        (state, tmp_dir)
    }

    #[test]
    fn finish_none() {
        let (mut state, _tmp_dir) = dummy_vm_state();
        let res = state.finish_exec(None);
        assert!(res.is_ok());
        assert!(res.unwrap().is_empty(), "no logs should have been written");
    }

    #[test]
    fn finish_resets_everything() {
        let (mut state, _tmp_dir) = dummy_vm_state();
        let registers = [
            REGISTER_OUTPUT,
            REGISTER_CURSOR + 1,
            REGISTER_OUTPUT_HTTP_RESULT,
            REGISTER_OUTPUT_HTTP_STATUS,
        ];
        for r in registers {
            state.set_register(r, vec![1, 2, 3, 4]);
        }
        let _res = state.finish_exec(None);
        for r in registers {
            assert!(state.get_register(r).is_none(), "registers must be cleared");
        }
        assert!(
            matches!(state.active, ActiveEntity::None),
            "active item must be reset"
        );
    }

    #[test]
    fn no_storage_key_on_inactive() {
        let (state, _tmp_dir) = dummy_vm_state();
        let res = state.get_storage_key(0, 0);
        assert!(res.is_err());
    }

    #[test]
    fn storage_key_prefix_matches_active() {
        let (mut state, _tmp_dir) = dummy_vm_state();
        // Check that storage-key-prefix matches contract-id
        let cid = ContractId::generate();
        state.active = ActiveEntity::contract_http(cid);
        let res = state.get_storage_key(0, 0);
        assert!(res.is_ok());
        let key_cid = res.unwrap().contract_id();
        assert!(key_cid.is_some());
        assert_eq!(key_cid.unwrap(), cid);

        // ...same with agent-id
        let aid = AgentId::generate();
        state.active = ActiveEntity::agent(aid, false);
        let res = state.get_storage_key(0, 0);
        assert!(res.is_ok());
        let key_aid = res.unwrap().agent_id();
        assert!(key_aid.is_some());
        assert_eq!(key_aid.unwrap(), aid);
    }
}
