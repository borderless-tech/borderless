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
        pub use off_chain::EnvInstance;
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

    fn storage_gen_sub_key() -> u64;

    fn storage_cursor(base_key: u64) -> u64;

    fn rand(min: u64, max: u64) -> u64;
}

pub trait OnInstance: StorageHandler {
    fn on_instance<F, R>(f: F) -> R
    where
        F: FnOnce(&mut Self) -> R;
}
