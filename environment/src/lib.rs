pub mod api;
// Re-export symbols to flatten the API
pub use api::*;

use cfg_if::cfg_if;
use serde::de::DeserializeOwned;
use serde::Serialize;

cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        // Use on_chain environment
        mod on_chain;
        pub use on_chain::EnvInstance;
    } else {
        // Use off_chain environment
        mod off_chain;
    }
}

pub trait StorageHandler {
    fn read_field<Value>(&self, base_key: u64, sub_key: u64) -> Option<Value>
    where
        Value: DeserializeOwned;

    fn write_field<Value>(&mut self, base_key: u64, sub_key: u64, value: &Value)
    where
        Value: Serialize;

    fn storage_remove(&mut self, base_key: u64, sub_key: u64);

    fn storage_has_key(&self, base_key: u64, sub_key: u64) -> bool;

    fn storage_gen_sub_key(&self) -> u64;

    fn storage_cursor(&mut self, base_key: u64) -> u64;

    fn rand(&self, min: u64, max: u64) -> u64;

    fn read_register(&self, register_id: u64) -> Option<Vec<u8>>;
}

pub trait OnInstance: StorageHandler {
    fn on_instance<F, R>(f: F) -> R
    where
        F: FnOnce(&mut Self) -> R;
}
