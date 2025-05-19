use borderless_abi as abi;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Register used for atomic operations
pub(crate) const REGISTER_ATOMIC_OP: u64 = u64::MAX - 1;

// The on_chain environment.
pub fn read_field<Value>(base_key: u64, sub_key: u64) -> Option<Value>
where
    Value: DeserializeOwned,
{
    let bytes = storage_read(base_key, sub_key)?;
    let value = match postcard::from_bytes::<Value>(&bytes) {
        Ok(value) => value,
        Err(_) => {
            //error!("SYSTEM: read-field failed base-key={base_key:x} sub-key={sub_key:x}: {e}");
            abort()
        }
    };
    Some(value)
}

pub fn write_field<Value>(base_key: u64, sub_key: u64, value: &Value)
where
    Value: Serialize,
{
    let value = match postcard::to_allocvec::<Value>(value) {
        Ok(value) => value,
        Err(_) => {
            //error!("SYSTEM: write-field failed base-key={base_key:x} sub-key={sub_key:x}: {e}");
            abort()
        }
    };
    storage_write(base_key, sub_key, value);
}

pub fn storage_remove(base_key: u64, sub_key: u64) {
    unsafe {
        abi::storage_remove(base_key, sub_key);
    }
}

pub fn storage_has_key(base_key: u64, sub_key: u64) -> bool {
    unsafe {
        match abi::storage_has_key(base_key, sub_key) {
            0 => false,
            1 => true,
            _ => {
                //error!("SYSTEM: invalid return code in 'storage_has_key' func");
                abort()
            }
        }
    }
}

pub fn storage_gen_sub_key() -> u64 {
    unsafe { abi::storage_gen_sub_key() }
}

pub fn storage_cursor(base_key: u64) -> u64 {
    unsafe { abi::storage_cursor(base_key) }
}

pub fn rand(min: u64, max: u64) -> u64 {
    unsafe { abi::rand(min, max) }
}

#[allow(clippy::uninit_vec)]
pub fn read_register(register_id: u64) -> Option<Vec<u8>> {
    unsafe {
        let len = register_len(register_id)?;
        let mut buf = Vec::with_capacity(len as usize);
        buf.set_len(len as usize);
        abi::read_register(register_id, buf.as_mut_ptr() as _);
        Some(buf)
    }
}

pub fn write_register(register_id: u64, data: impl AsRef<[u8]>) {
    unsafe {
        abi::write_register(
            register_id,
            data.as_ref().as_ptr() as _,
            data.as_ref().len() as _,
        );
    }
}

pub fn storage_read(base_key: u64, sub_key: u64) -> Option<Vec<u8>> {
    unsafe {
        abi::storage_read(base_key, sub_key, REGISTER_ATOMIC_OP);
    }
    read_register(REGISTER_ATOMIC_OP)
}

pub fn storage_write(base_key: u64, sub_key: u64, value: impl AsRef<[u8]>) {
    let value = value.as_ref();
    unsafe {
        abi::storage_write(base_key, sub_key, value.as_ptr() as _, value.len() as _);
    }
}

fn register_len(register_id: u64) -> Option<u64> {
    unsafe {
        let len = abi::register_len(register_id);
        // Check, if the register exists
        if len == u64::MAX {
            None
        } else {
            Some(len)
        }
    }
}

pub fn abort() -> ! {
    core::arch::wasm32::unreachable()
}
