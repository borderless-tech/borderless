use crate::__private::REGISTER_CURSOR;
use core::cell::RefCell;
use nohash_hasher::IntMap;
use rand::Rng;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;

// The off_chain environment
thread_local! {
    /// Simulates a Database
    pub static DATABASE: RefCell<HashMap<Vec<u8>, Vec<u8>>> = RefCell::new(HashMap::new());
}

thread_local! {
    /// Simulates the WASM memory
    pub static REGISTERS: RefCell<IntMap<u64, Vec<u8>>> = RefCell::new(IntMap::default());
}

pub fn read_field<Value>(base_key: u64, sub_key: u64) -> Option<Value>
where
    Value: DeserializeOwned,
{
    storage_read(base_key, sub_key).and_then(|bytes| {
        postcard::from_bytes::<Value>(bytes.as_slice()).ok() // TODO Handle error?
    })
}

pub fn write_field<Value>(base_key: u64, sub_key: u64, value: &Value)
where
    Value: Serialize,
{
    match postcard::to_allocvec::<Value>(value) {
        Ok(bytes) => storage_write(base_key, sub_key, bytes),
        Err(_) => panic!("Serialization error"),
    }
}

pub fn storage_remove(base_key: u64, sub_key: u64) {
    let key = calc_storage_key(base_key, sub_key);
    DATABASE.with(|db| {
        let mut db = db.borrow_mut();
        db.remove(&key);
    })
}

pub fn storage_has_key(base_key: u64, sub_key: u64) -> bool {
    let key = calc_storage_key(base_key, sub_key);
    DATABASE.with(|db| {
        let db = db.borrow();
        db.contains_key(&key)
    })
}

pub fn storage_gen_sub_key() -> u64 {
    rand(0, u64::MAX)
}

pub fn storage_cursor(base_key: u64) -> u64 {
    DATABASE.with(|db| {
        let mut db = db.borrow_mut();
        REGISTERS.with(|registers| {
            let mut registers = registers.borrow_mut();

            // Discard the HashMap's number of elements
            let sub_key = calc_storage_key(base_key, 0);
            db.remove(&sub_key);

            // Clear registers content
            registers.retain(|&k, _| k < REGISTER_CURSOR);

            // Dump database content into registers starting at position REGISTER_CURSOR
            for (i, (bytes, _)) in db.iter().enumerate() {
                // Extract the sub-key bytes
                let mut bytes: [u8; 8] = bytes[8..16].try_into().unwrap();
                // Reinterpret bytes as little-endian
                bytes.reverse();
                // Insert sub-key into registers
                registers.insert(REGISTER_CURSOR.saturating_add(i as u64), bytes.to_vec());
            }
        });

        // Return number of elements in DB
        db.len() as u64
    })
}

pub fn storage_read(base_key: u64, sub_key: u64) -> Option<Vec<u8>> {
    let key = calc_storage_key(base_key, sub_key);
    DATABASE.with(|db| {
        let db = db.borrow();
        db.get(&key).cloned()
    })
}

pub fn storage_write(base_key: u64, sub_key: u64, value: impl AsRef<[u8]>) {
    let key = calc_storage_key(base_key, sub_key);
    DATABASE.with(|db| {
        let mut db = db.borrow_mut();
        db.insert(key, value.as_ref().to_vec());
    })
}

pub fn rand(min: u64, max: u64) -> u64 {
    let mut range = rand::rng();
    range.random_range(min..max)
}

pub fn read_register(register_id: u64) -> Option<Vec<u8>> {
    REGISTERS.with(|registers| {
        let registers = registers.borrow();
        registers.get(&register_id).cloned()
    })
}

pub fn write_register(register_id: u64, data: impl AsRef<[u8]>) {
    REGISTERS.with(|registers| {
        let mut registers = registers.borrow_mut();
        registers.insert(register_id, data.as_ref().to_vec())
    });
}

pub fn register_len(register_id: u64) -> Option<u64> {
    read_register(register_id).map(|register| register.len() as u64)
}

/// Calculates a storage key from base key, and sub key (ignoring the contract-id).
pub fn calc_storage_key(base_key: u64, sub_key: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(16);
    out.extend_from_slice(&base_key.to_be_bytes());
    out.extend_from_slice(&sub_key.to_be_bytes());
    out
}

pub fn abort() -> ! {
    std::process::abort()
}
