use serde::de::DeserializeOwned;
use serde::Serialize;

pub fn read_field<Value>(base_key: u64, sub_key: u64) -> Option<Value>
where
    Value: DeserializeOwned,
{
    super::off_chain::read_field(base_key, sub_key)
}

pub fn write_field<Value>(base_key: u64, sub_key: u64, value: &Value)
where
    Value: Serialize,
{
    super::off_chain::write_field(base_key, sub_key, value)
}

pub fn storage_remove(base_key: u64, sub_key: u64) {
    super::off_chain::storage_remove(base_key, sub_key)
}

pub fn storage_has_key(base_key: u64, sub_key: u64) -> bool {
    super::off_chain::storage_has_key(base_key, sub_key)
}

pub fn storage_gen_sub_key() -> u64 {
    super::off_chain::storage_gen_sub_key()
}

pub fn storage_cursor(base_key: u64) -> u64 {
    super::off_chain::storage_cursor(base_key)
}

pub fn rand(min: u64, max: u64) -> u64 {
    super::off_chain::rand(min, max)
}

pub fn read_register(register_id: u64) -> Option<Vec<u8>> {
    super::off_chain::read_register(register_id)
}
