#![allow(unused)]
//! Contains the implementation of the ABI

use std::{cell::RefCell, time::Instant};

use borderless_sdk::ContractId;
use wasmtime::{Caller, Extern, Memory};

use log::{debug, error, info, trace, warn};
use nohash::IntMap;
use rand::{random, Rng};

use borderless_kv_store::*;

pub struct VmState<'a, S: Db> {
    registers: IntMap<u32, RefCell<Vec<u8>>>,
    db: &'a S,
    db_ptr: S::Handle,
    db_acid_txn: Option<<S as Db>::RwTx<'a>>,
    last_timer: Option<Instant>,

    // Currently active contract
    active_contract: Option<ContractId>,
}

impl<'a, S: Db> VmState<'a, S> {
    pub fn new(db: &'a S, db_ptr: S::Handle) -> Self {
        VmState {
            registers: Default::default(),
            db,
            db_ptr,
            db_acid_txn: None.into(),
            // _marker: PhantomData,
            last_timer: None,
            active_contract: None,
        }
    }

    // Set a new contract as active
    pub fn set_contract(&mut self, contract_id: ContractId) {
        self.active_contract = Some(contract_id);
    }

    // Resets the contract state
    pub fn reset_contract(&mut self) {
        self.active_contract = None;
    }

    fn get_storage_key(&self, base_key: u32, sub_key: u32) -> wasmtime::Result<[u8; 16]> {
        match &self.active_contract {
            Some(cid) => {
                // Prepare storage key
                let mut out = [0u8; 16];
                // The first 16 bytes are the contract-id
                out[0..16].copy_from_slice(cid.as_ref());
                // XOR the contract-id to shorten it
                for i in 0..8 {
                    out[i] = out[i] ^ out[i + 8];
                }
                // Then the field key (aka base-key)
                out[8..12].copy_from_slice(&base_key.to_be_bytes());
                // Then the sub-field key (aka sub-key)
                out[12..16].copy_from_slice(&sub_key.to_be_bytes());
                Ok(out)
            }
            None => Err(wasmtime::Error::msg("no contract has been activated")),
        }
    }

    pub fn set_register(&mut self, register_id: u32, value: Vec<u8>) {
        self.registers.insert(register_id, value.into());
    }

    pub fn get_register(&self, register_id: u32) -> Option<Vec<u8>> {
        self.registers.get(&register_id).map(|v| v.borrow().clone())
    }

    // NOTE: If there are two acid transactions, this is a caller error, and not a runtime error.
    pub fn begin_acid_txn(&mut self) -> wasmtime::Result<u32> {
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
    pub fn commit_acid_txn(&mut self) -> wasmtime::Result<u32> {
        assert!(
            self.active_contract.is_some(),
            "transactions should only be created when there is an active contract"
        );
        let now = Instant::now();
        match std::mem::replace(&mut self.db_acid_txn, None) {
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
}

// Helper function to get the memory of the wasm module
fn get_memory(caller: &mut Caller<'_, VmState<impl Db>>) -> wasmtime::Result<Memory> {
    match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => Ok(mem),
        _ => Err(wasmtime::Error::msg("Failed to find memory")),
    }
}

// Helper function to create a Vec<u8> that serves as a buffer with given length
fn create_buffer(len: u32) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(len as usize);
    unsafe {
        buffer.set_len(len as usize);
    }
    buffer
}

// --- Begin to implement abi
pub fn tic(mut caller: Caller<'_, VmState<impl Db>>) {
    caller.data_mut().last_timer = Some(Instant::now());
}

pub fn toc(caller: Caller<'_, VmState<impl Db>>) -> u64 {
    if let Some(timer) = caller.data().last_timer {
        let elapsed = timer.elapsed();
        elapsed
            .as_nanos()
            .try_into()
            .expect("your program should not run for 584.942 years")
    } else {
        panic!("-- no timer present");
    }
}

pub fn print(
    mut caller: Caller<'_, VmState<impl Db>>,
    ptr: u32,
    len: u32,
    level: u32,
) -> wasmtime::Result<()> {
    // Read string from WASM memory and print it
    // (Implementation details omitted for brevity)
    // let s = String::from_raw_parts(, length, capacity)
    let memory = get_memory(&mut caller)?;

    // Read memory
    let data = memory
        .data(&mut caller)
        .get(ptr as usize..(ptr + len) as usize)
        .ok_or_else(|| wasmtime::Error::msg("Memory access out of bounds"))?;

    let s =
        String::from_utf8(data.to_vec()).unwrap_or_else(|e| format!("Invalid UTF-8 sequence: {e}"));

    match level {
        0 => trace!("{s}"),
        1 => debug!("{s}"),
        2 => info!("{s}"),
        3 => warn!("{s}"),
        4 => error!("{s}"),
        _ => panic!("{s}"), // this should not happen
    }

    Ok(())
}

pub fn read_register(
    mut caller: Caller<'_, VmState<impl Db>>,
    register_id: u32,
    ptr: u32,
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

pub fn register_len(caller: Caller<'_, VmState<impl Db>>, register_id: u32) -> u32 {
    match caller.data().registers.get(&register_id) {
        Some(data) => data.borrow().len() as u32,
        None => u32::MAX,
    }
}

pub fn write_register(
    mut caller: Caller<'_, VmState<impl Db>>,
    register_id: u32,
    wasm_ptr: u32,
    wasm_ptr_len: u32,
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
    base_key: u32,
    sub_key: u32,
    value_ptr: u32,
    value_len: u32,
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
    base_key: u32,
    sub_key: u32,
    register_id: u32,
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
        return Err(wasmtime::Error::msg(
            "value not found: base_key={base_key}, sub_key={sub_key}",
        ));
    }

    let elapsed = now.elapsed();
    debug!("storage-read: {:?}", elapsed);
    Ok(())
}

pub fn storage_remove(
    mut caller: Caller<'_, VmState<impl Db>>,
    base_key: u32,
    sub_key: u32,
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
    base_key: u32,
    sub_key: u32,
) -> wasmtime::Result<u32> {
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
    Ok(result as u32)
}

pub fn storage_random_key() -> wasmtime::Result<u32> {
    let mut rng = rand::rng();
    let value: u32 = rng.random();
    // Add 1 unit to avoid generating a 0
    Ok(value.saturating_add(1))
}

pub fn rand(min: u32, max: u32) -> wasmtime::Result<u32> {
    let mut rng = rand::rng();
    let value: u32 = rng.random_range(min..max);
    Ok(value)
}

// NOTE: If there are two acid transactions, this is a caller error, and not a runtime error.
pub fn storage_begin_acid_txn(mut caller: Caller<'_, VmState<impl Db>>) -> wasmtime::Result<u32> {
    caller.data_mut().begin_acid_txn()
}

// NOTE: If there are two acid transactions, this is a caller error, and not a runtime error.
pub fn storage_commit_acid_txn(mut caller: Caller<'_, VmState<impl Db>>) -> wasmtime::Result<u32> {
    caller.data_mut().commit_acid_txn()
}
