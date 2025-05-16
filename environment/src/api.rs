use crate::{EnvInstance, OnInstance, StorageHandler};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub fn read_field<Value>(base_key: u64, sub_key: u64) -> Option<Value>
where
    Value: DeserializeOwned,
{
    EnvInstance::on_instance(|instance| StorageHandler::read_field(instance, base_key, sub_key))
}

pub fn write_field<Value>(base_key: u64, sub_key: u64, value: &Value)
where
    Value: Serialize,
{
    EnvInstance::on_instance(|instance| {
        StorageHandler::write_field(instance, base_key, sub_key, value)
    })
}

pub fn storage_remove(base_key: u64, sub_key: u64) {
    EnvInstance::on_instance(|instance| StorageHandler::storage_remove(instance, base_key, sub_key))
}

pub fn storage_has_key(base_key: u64, sub_key: u64) -> bool {
    EnvInstance::on_instance(|instance| {
        StorageHandler::storage_has_key(instance, base_key, sub_key)
    })
}

pub fn storage_gen_sub_key() -> u64 {
    EnvInstance::on_instance(|instance| StorageHandler::storage_gen_sub_key(instance))
}

pub fn storage_cursor(base_key: u64) -> u64 {
    EnvInstance::on_instance(|instance| StorageHandler::storage_cursor(instance, base_key))
}

pub fn rand(min: u64, max: u64) -> u64 {
    <EnvInstance as OnInstance>::on_instance(|instance| StorageHandler::rand(instance, min, max))
}

pub fn read_register(register_id: u64) -> Option<Vec<u8>> {
    EnvInstance::on_instance(|instance| StorageHandler::read_register(instance, register_id))
}
