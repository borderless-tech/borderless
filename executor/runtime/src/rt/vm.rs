//! WASM Virtual Machine
//!
//! This module contains the state of the virtual machine, that is shared across host function invocations,
//! and the concrete implementation of the ABI host functions, that are linked to the webassembly module by the runtime.

use std::{
    cell::RefCell,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use borderless::{
    __private::storage_keys::StorageKey,
    contracts::{Introduction, Revocation, TxCtx},
    events::CallAction,
    log::LogLine,
    AgentId, ContractId,
};
use wasmtime::{Caller, Extern, Memory};

use log::{debug, warn};
use nohash::IntMap;
use rand::Rng;

use borderless_kv_store::*;

use crate::{
    controller::{write_introduction, write_revocation, Controller},
    error::ErrorKind,
    rt::action_log::{ActionLog, ActionRecord},
    rt::logger::Logger,
    Error, Result,
};

// NOTE: I think this generalizes for both contracts and sw-agents;
//
// We have to fine-tune some things, but in general this works.

pub struct VmState<S: Db> {
    registers: IntMap<u64, RefCell<Vec<u8>>>,
    db: S,
    db_ptr: S::Handle,
    db_acid_txn_buffer: Option<Vec<StorageOp>>,
    last_timer: Option<Instant>,

    /// Current buffer of log output for the given contract
    log_buffer: Vec<LogLine>,

    // Currently active contract or sw-agent
    active_item: ActiveItem,
}

impl<S: Db> VmState<S> {
    pub fn new(db: S, db_ptr: S::Handle) -> Self {
        VmState {
            registers: Default::default(),
            db,
            db_ptr,
            db_acid_txn_buffer: None,
            last_timer: None,
            log_buffer: Vec::new(),
            active_item: ActiveItem::None,
        }
    }

    /// Sets an contract as active and marks it as mutable
    ///
    /// # Errors
    ///
    /// Calling this function while the `VmState` already has an active contract results in an error.
    pub fn begin_mutable_exec(&mut self, cid: ContractId) -> Result<()> {
        if self.active_item.is_some() {
            return Err(Error::msg(
                "Cannot start a new execution while something else is still active",
            ));
        }
        if Controller::new(&self.db).contract_revoked(&cid)? {
            return Err(ErrorKind::RevokedContract { cid }.into());
        }
        self.active_item = ActiveItem::Contract { cid, mutable: true };
        self.log_buffer.clear();
        self.db_acid_txn_buffer = Some(Vec::new());
        Ok(())
    }

    /// Marks the end of a mutable contract execution.
    ///
    /// Internally, this function does the following things:
    /// 1. Flush the log-buffer to the database
    /// 2. Reset the contract-id for the next execution
    /// 3. Clear the log-buffer
    ///
    /// # Errors
    ///
    /// Calling this function while the `VmState` has no active contract results in an error.
    pub fn finish_mutable_exec(&mut self, commit: Commit) -> Result<()> {
        let result = self.finish_mut_exec_inner(commit);

        // Reset everything
        self.active_item = ActiveItem::None;
        self.log_buffer.clear();

        result
    }

    // Inner wrapper, so we can return the result, but also perform the cleanup afterwards in case of an error
    fn finish_mut_exec_inner(&mut self, commit: Commit) -> Result<()> {
        let cid = match self.active_item {
            ActiveItem::Contract { cid, mutable } => {
                if !mutable {
                    return Err(Error::msg("Contract execution was marked as immutable"));
                }
                cid
            }
            ActiveItem::Agent { .. } => {
                return Err(Error::msg(
                    "Cannot finish a contract while a sw-agent is active",
                ));
            }
            ActiveItem::None => {
                return Err(Error::msg("No active contract"));
            }
        };
        let now = Instant::now();

        // Commit storage buffer
        let buf = std::mem::take(&mut self.db_acid_txn_buffer).unwrap_or_default();
        let mut txn = self.db.begin_rw_txn()?;
        for op in buf.into_iter() {
            // Check, that all keys are user-keys - ignore system-keys.
            if !op.is_userspace() {
                warn!("Contract tried to write or remove a value with a storage-key that is not in user-space");
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
            Commit::Action { action, tx_ctx } => {
                let action_log = ActionLog::new(&self.db, cid);
                action_log.commit(&self.db_ptr, &mut txn, &action, tx_ctx)?;
            }
            Commit::Introduction {
                mut introduction,
                tx_ctx,
            } => {
                assert_eq!(introduction.contract_id, cid);
                introduction.meta.active_since = timestamp;
                introduction.meta.tx_ctx_introduction = Some(tx_ctx);
                write_introduction::<S>(&self.db_ptr, &mut txn, &introduction)?;
            }
            Commit::Revocation { revocation, tx_ctx } => {
                assert_eq!(revocation.contract_id, cid);
                write_revocation::<S>(&self.db_ptr, &mut txn, &revocation, tx_ctx, timestamp)?;
            }
        }

        // Flush log
        let logger = Logger::new(&self.db, cid);
        logger.flush_lines(&self.log_buffer, &self.db_ptr, &mut txn)?;

        // Commit txn
        txn.commit()?;

        let elapsed = now.elapsed();
        debug!("commit-acid-txn: {elapsed:?}");
        Ok(())
    }

    /// Sets an contract as active and marks it as immutable
    ///
    /// This is used for handling http-requests (as they never modify the state)
    /// or for performing dry-runs (to check if an action *could* be executed without errors).
    ///
    /// # Errors
    ///
    /// Calling this function while the `VmState` already has an active contract results in an error.
    pub fn begin_immutable_exec(&mut self, cid: ContractId) -> Result<()> {
        if self.active_item.is_some() {
            return Err(Error::msg("Cannot overwrite active contract"));
        }
        self.active_item = ActiveItem::Contract {
            cid,
            mutable: false,
        };
        self.log_buffer.clear();
        self.db_acid_txn_buffer = None;
        Ok(())
    }

    /// Marks the end of an immutable contract execution.
    ///
    /// Can also be called to clear the `VmState` in case a mutable execution produced an error.
    ///
    /// Internally, this function does the following things:
    /// 1. Reset the contract-id for the next execution
    /// 2. Clear the log-buffer
    ///
    /// Please note: No logs are commited to the database.
    ///
    /// # Errors
    ///
    /// Calling this function while the `VmState` has no active contract results in an error.
    pub fn finish_immutable_exec(&mut self) -> Result<Vec<LogLine>> {
        if self.active_item.is_none() {
            return Err(Error::msg("Cannot clear non existing contract or sw-agent"));
        }
        self.active_item = ActiveItem::None;
        self.db_acid_txn_buffer = None;
        let log_output = std::mem::take(&mut self.log_buffer);
        Ok(log_output)
    }

    pub fn begin_agent_exec(&mut self, aid: AgentId, mutable: bool) -> Result<()> {
        if self.active_item.is_some() {
            return Err(Error::msg(
                "Cannot start a new execution while something else is still active",
            ));
        }
        self.active_item = ActiveItem::Agent { aid, mutable };
        self.log_buffer.clear();
        self.db_acid_txn_buffer = None;
        Ok(())
    }

    pub fn finish_agent_exec(&mut self, commit_state: bool) -> Result<Vec<LogLine>> {
        match self.active_item {
            ActiveItem::Contract { .. } => {
                return Err(Error::msg(
                    "cannot finish an agent while a contract is still running",
                ))
            }
            ActiveItem::Agent { mutable, .. } => {
                if !mutable && commit_state {
                    return Err(Error::msg("Agent execution was marked as immutable"));
                }
            }
            ActiveItem::None => {
                return Err(Error::msg("No active sw-agent"));
            }
        }
        self.active_item = ActiveItem::None;
        self.db_acid_txn_buffer = None;
        let log_output = std::mem::take(&mut self.log_buffer);
        Ok(log_output)
    }

    /// Generates the storage key based on the currently active contract.
    ///
    /// Note: This function only generates user-keys, as values with system-keys must be commited by the host.
    fn get_storage_key(&self, base_key: u64, sub_key: u64) -> wasmtime::Result<StorageKey> {
        self.active_item
            .storage_key(base_key, sub_key)
            .ok_or_else(|| wasmtime::Error::msg("no contract has been activated"))
    }

    /// Writes the given value into the register.
    pub fn set_register(&mut self, register_id: u64, value: Vec<u8>) {
        self.registers.insert(register_id, value.into());
    }

    /// Returns a value from a register
    pub fn get_register(&self, register_id: u64) -> Option<Vec<u8>> {
        self.registers.get(&register_id).map(|v| v.borrow().clone())
    }

    /// Removes a value from a register
    pub fn clear_register(&mut self, register_id: u64) {
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
}

// Helper function to get the memory of the wasm module
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

// --- Begin to implement abi
pub fn tic(mut caller: Caller<'_, VmState<impl Db>>) {
    caller.data_mut().last_timer = Some(Instant::now());
}

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
pub fn print(
    mut caller: Caller<'_, VmState<impl Db>>,
    ptr: u64,
    len: u64,
    level: u32,
) -> wasmtime::Result<()> {
    // Get timestamp as early as possible
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("now > 1970")
        .as_nanos();

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

pub fn register_len(caller: Caller<'_, VmState<impl Db>>, register_id: u64) -> u64 {
    match caller.data().registers.get(&register_id) {
        Some(data) => data.borrow().len() as u64,
        None => u64::MAX,
    }
}

pub fn write_register(
    mut caller: Caller<'_, VmState<impl Db>>,
    register_id: u64,
    wasm_ptr: u64,
    wasm_ptr_len: u64,
) -> wasmtime::Result<()> {
    // Get memory
    let memory = get_memory(&mut caller)?;

    // Create buffer
    let mut buffer = create_buffer(wasm_ptr_len);

    // Read from memory
    memory
        .read(&mut caller, wasm_ptr as usize, &mut buffer)
        .map_err(|e| wasmtime::Error::msg(format!("Failed to read from memory: {e}")))?;
    // Write register
    caller.data_mut().set_register(register_id, buffer);
    Ok(())
}

// --- Storage api

pub fn storage_write(
    mut caller: Caller<'_, VmState<impl Db>>,
    base_key: u64,
    sub_key: u64,
    value_ptr: u64,
    value_len: u64,
) -> wasmtime::Result<()> {
    if caller.data().active_item.is_immutable() {
        return Ok(());
    }
    // Get memory
    let memory = get_memory(&mut caller)?;

    // Create buffers
    let mut value = create_buffer(value_len);

    // Read from memory
    memory.read(&mut caller, value_ptr as usize, &mut value)?;

    // Build key
    let key = caller.data().get_storage_key(base_key, sub_key)?;

    // Check, if there is an acid txn, and if so, commit the changes to that:
    let caller_data = &mut caller.data_mut();
    if let Some(buf) = &mut caller_data.db_acid_txn_buffer {
        buf.push(StorageOp::write(key, value));
    }
    Ok(())
}

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

pub fn storage_remove(
    mut caller: Caller<'_, VmState<impl Db>>,
    base_key: u64,
    sub_key: u64,
) -> wasmtime::Result<()> {
    if caller.data().active_item.is_immutable() {
        return Ok(());
    }

    // Build key
    let key = caller.data().get_storage_key(base_key, sub_key)?;

    // Check, if there is an acid txn, and if so, commit the changes to that:
    let caller_data = &mut caller.data_mut();

    // Write changes to storage-buffer
    if let Some(buf) = &mut caller_data.db_acid_txn_buffer {
        // txn.delete(&caller_data.db_ptr, &key)?;
        buf.push(StorageOp::remove(key));
    }
    Ok(())
}

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

pub fn storage_gen_sub_key() -> wasmtime::Result<u64> {
    let mut rng = rand::rng();
    Ok(rng.random())
}

pub fn rand(min: u64, max: u64) -> wasmtime::Result<u64> {
    let mut rng = rand::rng();
    let value: u64 = rng.random_range(min..max);
    Ok(value)
}

/// External data that must be commited in the contract
pub enum Commit {
    Action {
        action: CallAction,
        tx_ctx: TxCtx,
    },
    Introduction {
        introduction: Introduction,
        tx_ctx: TxCtx,
    },
    Revocation {
        revocation: Revocation,
        tx_ctx: TxCtx,
    },
}

// TODO: Maybe add Agents here aswell ?
/// Represents the different states of an active contract
///
/// A contract can be executed with a mutable or immutable state.
/// Processing a chain-transaction requires a mutable state,
/// as this means the state of the contract can change and changes are written to the database.
///
/// An immutable contract execution is e.g. required for handling http-requests or performing dry-runs.
/// In such a case, calls to `storage_write` will result in an error.
enum ActiveItem {
    Contract { cid: ContractId, mutable: bool },
    Agent { aid: AgentId, mutable: bool },
    None,
}

impl ActiveItem {
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    pub fn is_none(&self) -> bool {
        matches!(self, ActiveItem::None)
    }

    pub fn storage_key(&self, base_key: u64, sub_key: u64) -> Option<StorageKey> {
        match self {
            ActiveItem::Contract { cid, .. } => Some(StorageKey::new(&cid, base_key, sub_key)),
            ActiveItem::Agent { aid, .. } => Some(StorageKey::new(&aid, base_key, sub_key)),
            ActiveItem::None => None,
        }
    }

    pub fn is_immutable(&self) -> bool {
        match self {
            ActiveItem::Contract { mutable, .. } | ActiveItem::Agent { mutable, .. } => *mutable,
            ActiveItem::None => false,
        }
    }
}

/// Enum that represents a storage operation
///
/// All storage operations are commited to the key-value-store
/// once the contract finished its execution.
enum StorageOp {
    Write { key: StorageKey, value: Vec<u8> },
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
