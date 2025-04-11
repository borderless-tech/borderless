#![allow(unused)]
//! Contains the implementation of the ABI

use std::{
    cell::RefCell,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use borderless_sdk::{
    __private::action_log::SUB_KEY_LOG_LEN,
    __private::storage_keys::{StorageKey, BASE_KEY_ACTION_LOG},
    contract::{ActionRecord, CallAction},
    log::LogLine,
    ContractId,
};
use wasmtime::{Caller, Extern, Memory};

use log::{debug, error, info, trace, warn};
use nohash::IntMap;
use rand::{random, Rng};

use borderless_kv_store::*;

use crate::logger::Logger;

// NOTE: There is an option to get rid of the lifetime and borrowing in the VmState (which is tbh. quite annoying);
// Instead of opening the transaction and using it to buffer the writes,
// we could create a Vec<T> to store all calls to storage_write and storage_remove, and then commit them, once the contract has finished.
//
// I am not sure, if this may cause an overhead for really large operations in wasm; but I mean, the lmdb transaction also has to buffer everything somewhere,
// so it seems like it's not a big difference (but ofc I don't know how lmdb works internally, so I may be completely wrong here)..

pub struct VmState<'a, S: Db> {
    registers: IntMap<u64, RefCell<Vec<u8>>>,
    db: &'a S,
    db_ptr: S::Handle,
    db_acid_txn: Option<<S as Db>::RwTx<'a>>,
    last_timer: Option<Instant>,

    /// Current buffer of log output for the given contract
    log_buffer: Vec<LogLine>,

    // Currently active contract
    active_contract: Option<ContractId>,
}

impl<'a, S: Db> VmState<'a, S> {
    pub fn new(db: &'a S, db_ptr: S::Handle) -> Self {
        VmState {
            registers: Default::default(),
            db,
            db_ptr,
            db_acid_txn: None,
            // _marker: PhantomData,
            last_timer: None,
            log_buffer: Vec::new(),
            active_contract: None,
        }
    }

    /// Marks the start of a new contract execution
    ///
    /// Internally, this function does the following things:
    /// 1. Clear the log-buffer
    /// 2. Remember the contract-id, so we can generate storage-keys
    pub fn begin_contract_execution(&mut self, contract_id: ContractId) -> anyhow::Result<()> {
        if self.active_contract.is_some() {
            return Err(anyhow::Error::msg(
                "Must finish contract execution before starting new",
            ));
        }
        self.active_contract = Some(contract_id);
        self.log_buffer.clear();
        Ok(())
    }

    /// Marks the end of a new contract execution
    ///
    /// Internally, this function does the following things:
    /// 1. Flush the log-buffer to the database
    /// 2. Clear the contract-id, so it can be reset next time
    /// 3. Clear the log-buffer
    pub fn finish_contract_execution(&mut self) -> anyhow::Result<()> {
        match self.active_contract {
            Some(cid) => {
                // TODO: The flushing takes 10 ms due to lmdb being lmdb..
                let logger = Logger::new(self.db, cid);
                logger.flush_lines(&self.log_buffer)?;
            }
            None => {
                return Err(anyhow::Error::msg(
                    "Must start contract execution before commiting",
                ));
            }
        }
        self.active_contract = None;
        self.log_buffer.clear();
        Ok(())
    }

    /// Generates the storage key based on the currently active contract.
    ///
    /// Note: Does not do any further checking, if the key is in user or system space!
    fn get_storage_key(&self, base_key: u64, sub_key: u64) -> wasmtime::Result<StorageKey> {
        self.active_contract
            .map(|cid| StorageKey::new(&cid, base_key, sub_key))
            .ok_or_else(|| wasmtime::Error::msg("no contract has been activated"))
    }

    pub fn set_register(&mut self, register_id: u64, value: Vec<u8>) {
        self.registers.insert(register_id, value.into());
    }

    pub fn get_register(&self, register_id: u64) -> Option<Vec<u8>> {
        self.registers.get(&register_id).map(|v| v.borrow().clone())
    }

    pub fn clear_register(&mut self, register_id: u64) {
        self.registers.remove(&register_id);
    }

    // NOTE: If there are two acid transactions, this is a caller error, and not a runtime error.
    fn begin_acid_txn(&mut self) -> wasmtime::Result<u64> {
        assert!(
            self.active_contract.is_some(),
            "transactions should only be created when there is an active contract"
        );
        if self.db_acid_txn.is_some() {
            return Ok(1);
        }
        let txn = self.db.begin_rw_txn()?;
        self.db_acid_txn = Some(txn);
        Ok(0)
    }

    // NOTE: If there are two acid transactions, this is a caller error, and not a runtime error.
    fn commit_acid_txn(&mut self) -> wasmtime::Result<u64> {
        assert!(
            self.active_contract.is_some(),
            "transactions should only be created when there is an active contract"
        );
        let now = Instant::now();
        match self.db_acid_txn.take() {
            Some(txn) => {
                txn.commit()?; // TODO: This guy is taking 10x the time of the entire module execution
                               // (maybe in production we don't block the wasm module here ?)
                let elapsed = now.elapsed();
                debug!("commit-acid-txn: {elapsed:?}");
            }
            None => {
                return Ok(1);
            }
        }
        Ok(0)
    }

    /// Tries to read the action with the given index for the currently active contract
    pub fn read_action(
        &self,
        cid: &ContractId,
        idx: usize,
    ) -> anyhow::Result<Option<ActionRecord>> {
        use borderless_sdk::__private::from_postcard_bytes;
        let storage_key = StorageKey::system_key(cid, BASE_KEY_ACTION_LOG, idx as u64);

        let txn = self.db.begin_ro_txn()?;
        let value = if let Some(bytes) = txn.read(&self.db_ptr, &storage_key)? {
            Some(from_postcard_bytes(bytes)?)
        } else {
            None
        };
        txn.commit()?;
        Ok(value)
    }

    /// Returns the length of all actions
    pub fn len_actions(&self, cid: &ContractId) -> anyhow::Result<Option<u64>> {
        use borderless_sdk::__private::from_postcard_bytes;
        let storage_key = StorageKey::system_key(cid, BASE_KEY_ACTION_LOG, SUB_KEY_LOG_LEN);
        let txn = self.db.begin_ro_txn()?;
        let value = if let Some(bytes) = txn.read(&self.db_ptr, &storage_key)? {
            Some(from_postcard_bytes(bytes)?)
        } else {
            None
        };
        Ok(value)
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
    let timer = caller.data().last_timer.context("no timer present")?;
    let elapsed = timer.elapsed();
    elapsed
        .as_nanos()
        .try_into()
        .context("your program should not run for 584.942 years")
}

pub fn timestamp() -> wasmtime::Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("timestamp < 1970")?
        .as_millis()
        .try_into()
        .context("u64 should fit for 584942417 years")
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
) -> Result<(), wasmtime::Error> {
    let now = Instant::now();
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

    let elapsed = now.elapsed();
    debug!("read-register: {:?}", elapsed);

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
    let now = Instant::now();
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

    let elapsed = now.elapsed();
    debug!("write-register: {:?}", elapsed);
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
    let now = Instant::now();
    // Get memory
    let memory = get_memory(&mut caller)?;

    // Create buffers
    let mut value = create_buffer(value_len);

    // Read from memory
    memory.read(&mut caller, value_ptr as usize, &mut value)?;

    // Build key
    let key = caller.data().get_storage_key(base_key, sub_key)?;

    // Check, if there is an acid txn, and if so, commit the changes to that:
    let mut caller_data = &mut caller.data_mut();
    if let Some(txn) = &mut caller_data.db_acid_txn {
        txn.write(&caller_data.db_ptr, &key, &value)?;
    } else {
        // If not, create a new transaction and instantly commit the changes
        let mut txn = caller_data.db.begin_rw_txn()?;
        txn.write(&caller_data.db_ptr, &key, &value)?;
        txn.commit()?;
    }

    let elapsed = now.elapsed();
    debug!("storage-write: {:?}", elapsed);
    Ok(())
}

pub fn storage_read(
    mut caller: Caller<'_, VmState<impl Db>>,
    base_key: u64,
    sub_key: u64,
    register_id: u64,
) -> wasmtime::Result<()> {
    let now = Instant::now();

    // Build key
    let key = caller.data().get_storage_key(base_key, sub_key)?;

    // Check, if there is an acid txn, and if so, commit the changes to that:
    let mut caller_data = &mut caller.data_mut();
    let value = if let Some(txn) = &mut caller_data.db_acid_txn {
        txn.read(&caller_data.db_ptr, &key)?.map(|v| v.to_vec())
    } else {
        // If not, create a new transaction and instantly commit the changes
        let txn = caller_data.db.begin_ro_txn()?;
        let value = txn.read(&caller_data.db_ptr, &key)?.map(|v| v.to_vec());
        txn.commit()?;
        value
    };
    if let Some(value) = value {
        // Write to register
        caller.data_mut().set_register(register_id, value);
    } else {
        // return Err(wasmtime::Error::msg(
        //     "value not found: base_key={base_key}, sub_key={sub_key}",
        // ));
        // TODO: I think this should not be an error, as the storage_read abi
        // tries to read the register, and if the register has no value, if will be handled there.
        // So I think the cleanest way is to clear the register and return Ok(())
        caller_data.clear_register(register_id);
    }

    let elapsed = now.elapsed();
    debug!("storage-read: {:?}", elapsed);
    Ok(())
}

pub fn storage_remove(
    mut caller: Caller<'_, VmState<impl Db>>,
    base_key: u64,
    sub_key: u64,
) -> wasmtime::Result<()> {
    let now = Instant::now();

    // Build key
    let key = caller.data().get_storage_key(base_key, sub_key)?;

    // Check, if there is an acid txn, and if so, commit the changes to that:
    let mut caller_data = &mut caller.data_mut();
    if let Some(txn) = &mut caller_data.db_acid_txn {
        txn.delete(&caller_data.db_ptr, &key)?;
    } else {
        // If not, create a new transaction and instantly commit the changes
        let mut txn = caller_data.db.begin_rw_txn()?;
        txn.delete(&caller_data.db_ptr, &key)?;
        txn.commit()?;
    }

    let elapsed = now.elapsed();
    debug!("storage-remove: {:?}", elapsed);
    Ok(())
}

pub fn storage_has_key(
    mut caller: Caller<'_, VmState<impl Db>>,
    base_key: u64,
    sub_key: u64,
) -> wasmtime::Result<u64> {
    let now = Instant::now();

    // Build key
    let key = caller.data().get_storage_key(base_key, sub_key)?;

    // Check, if there is an acid txn, and if so, commit the changes to that:
    let mut caller_data = &mut caller.data_mut();
    let result = if let Some(txn) = &mut caller_data.db_acid_txn {
        txn.read(&caller_data.db_ptr, &key)?.is_some()
    } else {
        // If not, create a new transaction
        let txn = caller_data.db.begin_ro_txn()?;
        let found_key = txn.read(&caller_data.db_ptr, &key)?.is_some();
        txn.commit()?;
        found_key
    };

    let elapsed = now.elapsed();
    debug!("storage-has-key: {:?}", elapsed);
    Ok(result as u64)
}

pub fn storage_gen_sub_key() -> wasmtime::Result<u64> {
    let mut rng = rand::rng();
    let value: u64 = rng.random();
    // Add 1 unit to avoid generating a 0
    Ok(value.saturating_add(1))
}

pub fn rand(min: u64, max: u64) -> wasmtime::Result<u64> {
    let mut rng = rand::rng();
    let value: u64 = rng.random_range(min..max);
    Ok(value)
}

// NOTE: If there are two acid transactions, this is a caller error, and not a runtime error.
pub fn storage_begin_acid_txn(mut caller: Caller<'_, VmState<impl Db>>) -> wasmtime::Result<u64> {
    caller.data_mut().begin_acid_txn()
}

// NOTE: If there are two acid transactions, this is a caller error, and not a runtime error.
pub fn storage_commit_acid_txn(mut caller: Caller<'_, VmState<impl Db>>) -> wasmtime::Result<u64> {
    caller.data_mut().commit_acid_txn()
}
