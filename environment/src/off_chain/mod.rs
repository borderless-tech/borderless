use crate::StorageHandler;
use rand::Rng;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;

/// The off_chain environment
pub struct EnvInstance {
    /// Simulates a Database
    database: HashMap<Vec<u8>, Vec<u8>>,
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

    fn storage_gen_sub_key() -> u64 {
        Self::rand(0, u64::MAX)
    }

    fn storage_cursor(base_key: u64) -> u64 {
        todo!()
    }

    fn rand(min: u64, max: u64) -> u64 {
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
