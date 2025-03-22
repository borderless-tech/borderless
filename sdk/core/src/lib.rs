pub mod contract;
pub mod registers;

pub use abi::LogLevel;
use borderless_abi as abi;
use registers::REGISTER_ATOMIC_OP;
use serde::{de::DeserializeOwned, Serialize};

pub mod serialize {
    pub use serde_json::from_value;
}

pub fn print(level: abi::LogLevel, msg: impl AsRef<str>) {
    unsafe {
        abi::print(
            msg.as_ref().as_ptr() as _,
            msg.as_ref().len() as _,
            level as u32,
        );
    }
}

pub fn register_len(register_id: u32) -> Option<u32> {
    unsafe {
        let len = abi::register_len(register_id);
        // Check, if the register exists
        if len == u32::MAX {
            None
        } else {
            Some(len)
        }
    }
}

pub fn read_register(register_id: u32) -> Option<Vec<u8>> {
    unsafe {
        let len = register_len(register_id)?;
        let mut buf = Vec::with_capacity(len as usize);
        buf.set_len(len as usize);
        abi::read_register(register_id, buf.as_mut_ptr() as _);
        Some(buf)
    }
}

pub fn read_string_from_register(register_id: u32) -> Option<String> {
    read_register(register_id).and_then(|bytes| String::from_utf8(bytes).ok())
}

pub fn write_register(register_id: u32, data: impl AsRef<[u8]>) {
    unsafe {
        abi::write_register(
            register_id,
            data.as_ref().as_ptr() as _,
            data.as_ref().len() as _,
        );
    }
}

pub fn write_string_to_register(register_id: u32, string: impl AsRef<str>) {
    write_register(register_id, string.as_ref());
}

pub fn storage_write(base_key: u32, sub_key: u32, value: impl AsRef<[u8]>) {
    let value = value.as_ref();
    unsafe {
        abi::storage_write(
            base_key,
            sub_key,
            value.as_ptr() as _,
            value.len() as _,
            REGISTER_ATOMIC_OP,
        );
    }
}

pub fn storage_read(base_key: u32, sub_key: u32) -> Option<Vec<u8>> {
    unsafe {
        abi::storage_read(base_key, sub_key, REGISTER_ATOMIC_OP);
    }
    read_register(REGISTER_ATOMIC_OP)
}

pub fn storage_remove(base_key: u32, sub_key: u32) {
    unsafe {
        abi::storage_remove(base_key, sub_key);
    }
}

pub fn storage_has_key(base_key: u32, sub_key: u32) -> bool {
    unsafe {
        match abi::storage_has_key(base_key, sub_key) {
            0 => false,
            1 => true,
            _ => abort(),
        }
    }
}

pub fn storage_random_key() -> u32 {
    unsafe { abi::storage_random_key() }
}

pub fn storage_begin_acid_txn() {
    unsafe {
        if abi::storage_begin_acid_txn() != 0 {
            abort()
        }
    }
}

pub fn storage_commit_acid_txn() {
    unsafe {
        if abi::storage_commit_acid_txn() != 0 {
            abort()
        }
    }
}

pub fn read_field<Value>(base_key: u32, sub_key: u32) -> Option<Value>
where
    Value: DeserializeOwned,
{
    let bytes = storage_read(base_key, sub_key)?;
    let value = postcard::from_bytes::<Value>(&bytes).unwrap();
    Some(value)
}

pub fn write_field<Value>(base_key: u32, sub_key: u32, value: &Value)
where
    Value: Serialize,
{
    let value = postcard::to_allocvec::<Value>(value).unwrap();
    storage_write(base_key, sub_key, value);
}

pub fn abort() -> ! {
    #[cfg(target_arch = "wasm32")]
    {
        core::arch::wasm32::unreachable()
    }
    #[cfg(not(target_arch = "wasm32"))]
    unsafe {
        abi::panic()
    }
}

pub mod dev {
    use std::time::Duration;

    use borderless_abi as abi;

    pub fn tic() {
        unsafe { abi::tic() }
    }

    pub fn tocp() {
        unsafe { abi::tocp() }
    }

    pub fn toc() -> Duration {
        let dur = unsafe { abi::toc() };
        Duration::from_nanos(dur)
    }

    pub fn rand(min: u32, max: u32) -> u32 {
        unsafe { abi::rand(min, max) }
    }
}
