use crate::{OnInstance, StorageHandler};
use nohash_hasher::IntMap;
use rand::Rng;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;

// Cursor content start from this register
pub const REGISTER_CURSOR: u64 = 2 << 32;

/// The off_chain environment
pub struct EnvInstance {
    /// Simulates a Database
    database: HashMap<Vec<u8>, Vec<u8>>,
    /// Simulates the WASM memory
    registers: IntMap<u64, Vec<u8>>,
}

impl EnvInstance {
    pub fn new() -> Self {
        Self {
            database: HashMap::new(),
            registers: IntMap::default(),
        }
    }
}

impl OnInstance for EnvInstance {
    fn on_instance<F, R>(f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        use core::cell::RefCell;

        thread_local!(
            static INSTANCE: RefCell<EnvInstance> = RefCell::new(
                EnvInstance::new()
            )
        );
        INSTANCE.with(|instance| f(&mut instance.borrow_mut()))
    }
}

impl StorageHandler for EnvInstance {
    fn read_field<Value>(&self, base_key: u64, sub_key: u64) -> Option<Value>
    where
        Value: DeserializeOwned,
    {
        let key = calc_storage_key(base_key, sub_key);

        self.database.get(&key).and_then(|bytes| {
            postcard::from_bytes::<Value>(&bytes).ok() // TODO Handle error?
        })
    }

    fn write_field<Value>(&mut self, base_key: u64, sub_key: u64, value: &Value)
    where
        Value: Serialize,
    {
        let key = calc_storage_key(base_key, sub_key);

        match postcard::to_allocvec::<Value>(value) {
            Ok(bytes) => {
                self.database.insert(key, bytes);
            }
            Err(_) => panic!("Serialization error"),
        }
    }

    fn storage_remove(&mut self, base_key: u64, sub_key: u64) {
        let key = calc_storage_key(base_key, sub_key);
        self.database.remove(&key);
    }

    fn storage_has_key(&self, base_key: u64, sub_key: u64) -> bool {
        let key = calc_storage_key(base_key, sub_key);
        self.database.contains_key(&key)
    }

    fn storage_gen_sub_key(&self) -> u64 {
        Self::rand(self, 0, u64::MAX)
    }

    fn storage_cursor(&mut self, base_key: u64) -> u64 {
        let _ = base_key; // Suppress unused warning

        // Dump database content into registers starting at position REGISTER_CURSOR
        let size = self.database.len();
        for i in 0..size {
            let value = self.database.iter().nth(i).unwrap().1;
            self.registers
                .insert(REGISTER_CURSOR.saturating_add(i as u64), value.clone());
        }
        size as u64
    }

    fn rand(&self, min: u64, max: u64) -> u64 {
        let mut range = rand::rng();
        range.random_range(min..max)
    }
}

/// Calculates a storage key from base key, and sub key (ignoring the contract-id).
pub fn calc_storage_key(base_key: u64, sub_key: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(16);
    out.extend_from_slice(&base_key.to_be_bytes());
    out.extend_from_slice(&sub_key.to_be_bytes());
    out
}
