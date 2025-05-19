use serde::de::DeserializeOwned;
use serde::Serialize;

pub fn read_field<Value>(base_key: u64, sub_key: u64) -> Option<Value>
where
    Value: DeserializeOwned,
{
    #[cfg(target_arch = "wasm32")]
    {
        super::on_chain::read_field(base_key, sub_key)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        super::off_chain::read_field(base_key, sub_key)
    }
}

pub fn write_field<Value>(base_key: u64, sub_key: u64, value: &Value)
where
    Value: Serialize,
{
    #[cfg(target_arch = "wasm32")]
    {
        super::on_chain::write_field(base_key, sub_key, value)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        super::off_chain::write_field(base_key, sub_key, value)
    }
}

pub fn storage_remove(base_key: u64, sub_key: u64) {
    #[cfg(target_arch = "wasm32")]
    {
        super::on_chain::storage_remove(base_key, sub_key)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        super::off_chain::storage_remove(base_key, sub_key)
    }
}

pub fn storage_has_key(base_key: u64, sub_key: u64) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        super::on_chain::storage_has_key(base_key, sub_key)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        super::off_chain::storage_has_key(base_key, sub_key)
    }
}

pub fn storage_gen_sub_key() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        super::on_chain::storage_gen_sub_key()
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        super::off_chain::storage_gen_sub_key()
    }
}

pub fn storage_cursor(base_key: u64) -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        super::on_chain::storage_cursor(base_key)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        super::off_chain::storage_cursor(base_key)
    }
}

pub fn rand(min: u64, max: u64) -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        super::on_chain::rand(min, max)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        super::off_chain::rand(min, max)
    }
}

pub fn read_register(register_id: u64) -> Option<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    {
        super::on_chain::read_register(register_id)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        super::off_chain::read_register(register_id)
    }
}
