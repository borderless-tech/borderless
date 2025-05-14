// We have to set the path explicitly, because the module is named "__private", while the directory is named "private"
#[path = "private/http.rs"]
pub mod http;
#[path = "private/registers.rs"]
pub mod registers;
#[path = "private/storage_keys.rs"]
pub mod storage_keys;
#[path = "private/storage_traits.rs"]
pub mod storage_traits;

use borderless_abi as abi;

use registers::*;
use serde::{de::DeserializeOwned, Serialize};

pub use postcard::from_bytes as from_postcard_bytes;
pub use postcard::to_allocvec as to_postcard_bytes;

use crate::{contracts::Introduction, error};

// NOTE: Maybe we can use conditional compilation, to guard all functions that must only be called from the webassembly code:
//
//    #[cfg(target_arch = "wasm32")]
//    {
//        core::arch::wasm32::unreachable()
//    }
//    #[cfg(not(target_arch = "wasm32"))]
//    unsafe {
//        panic!("this is not allowed!")
//    }
//
// Maybe we can utilize this in a way, that makes our wasm code testable ?
// Because without links to the abi, we cannot really test all this..
//

// --- PLAYGROUND FOR NEW ABI STUFF

pub fn send_http_rq(
    rq_head: impl AsRef<str>,
    rq_body: impl AsRef<[u8]>,
) -> Result<(String, Vec<u8>), String> {
    write_string_to_register(REGISTER_REQUEST_HEAD, rq_head);
    write_register(REGISTER_REQUEST_BODY, &rq_body);
    let result = unsafe {
        abi::send_http_rq(
            REGISTER_REQUEST_HEAD,
            REGISTER_REQUEST_BODY,
            REGISTER_RESPONSE_HEAD,
            REGISTER_RESPONSE_BODY,
            REGISTER_ATOMIC_OP,
        )
    };

    match result {
        0 => {
            let rs_head = required(
                read_string_from_register(REGISTER_RESPONSE_HEAD),
                "missing required response header",
            );
            let rs_body = required(
                read_register(REGISTER_RESPONSE_BODY),
                "missing required response body",
            );
            Ok((rs_head, rs_body))
        }
        1 => {
            let error = required(
                read_string_from_register(REGISTER_ATOMIC_OP),
                "missing required error message for failed responses",
            );
            Err(error)
        }
        _ => {
            error!("SYSTEM: invalid return code in 'send_http_rq' func");
            abort()
        }
    }
}

/// Helper function that marks a register value as "required" - meaning that in case the value is not present,
/// we abort the execution (this is an implementation error of the runtime, and not an error that the user should handle).
///
/// To avoid "being in the dark" when debugging this, we log an error message before aborting - just like we do with [`read_field`].
fn required<T, S: AsRef<str>>(value: Option<T>, msg: S) -> T {
    match value {
        Some(v) => v,
        None => {
            error!("SYSTEM: {}", msg.as_ref());
            abort();
        }
    }
}

// ---

pub fn print(level: abi::LogLevel, msg: impl AsRef<str>) {
    unsafe {
        abi::print(
            msg.as_ref().as_ptr() as _,
            msg.as_ref().len() as _,
            level as u32,
        );
    }
}

fn register_len(register_id: u64) -> Option<u64> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let len = abi::register_len(register_id);
        // Check, if the register exists
        if len == u64::MAX {
            None
        } else {
            Some(len)
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = register_id; // remove unused warning
        panic!("this method can just be called from within a wasm32 target")
    }
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

pub fn read_string_from_register(register_id: u64) -> Option<String> {
    read_register(register_id).and_then(|bytes| String::from_utf8(bytes).ok())
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

pub fn write_string_to_register(register_id: u64, string: impl AsRef<str>) {
    write_register(register_id, string.as_ref());
}

fn storage_write(base_key: u64, sub_key: u64, value: impl AsRef<[u8]>) {
    let value = value.as_ref();
    unsafe {
        abi::storage_write(base_key, sub_key, value.as_ptr() as _, value.len() as _);
    }
}

fn storage_read(base_key: u64, sub_key: u64) -> Option<Vec<u8>> {
    unsafe {
        abi::storage_read(base_key, sub_key, REGISTER_ATOMIC_OP);
    }
    read_register(REGISTER_ATOMIC_OP)
}

pub fn storage_remove(base_key: u64, sub_key: u64) {
    unsafe {
        abi::storage_remove(base_key, sub_key);
    }
}

pub fn storage_cursor(base_key: u64) -> u64 {
    unsafe { abi::storage_cursor(base_key) }
}

pub fn storage_has_key(base_key: u64, sub_key: u64) -> bool {
    unsafe {
        match abi::storage_has_key(base_key, sub_key) {
            0 => false,
            1 => true,
            _ => {
                error!("SYSTEM: invalid return code in 'storage_has_key' func");
                abort()
            }
        }
    }
}

pub fn storage_gen_sub_key() -> u64 {
    unsafe { abi::storage_gen_sub_key() }
}

/// Reads a value from the storage via the register.
///
/// Returns `None` if no value could be found at the given storage keys.
pub fn read_field<Value>(base_key: u64, sub_key: u64) -> Option<Value>
where
    Value: DeserializeOwned,
{
    let bytes = storage_read(base_key, sub_key)?;
    let value = match postcard::from_bytes::<Value>(&bytes) {
        Ok(value) => value,
        Err(e) => {
            error!("SYSTEM: read-field failed base-key={base_key:x} sub-key={sub_key:x}: {e}");
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
        Err(e) => {
            error!("SYSTEM: write-field failed base-key={base_key:x} sub-key={sub_key:x}: {e}");
            abort()
        }
    };
    storage_write(base_key, sub_key, value);
}

// TODO: Remove ! Introductions should be commited by the host !
/// Helper function, that stores all information from the contract-introduction in the key-value-storage
///
/// Must be called from the webassembly code ("client-side"), as it internally relies on [`write_field`].
pub fn write_metadata_client(introduction: &Introduction) {
    use storage_keys::*;

    // Write contract-id
    write_field(
        BASE_KEY_METADATA,
        META_SUB_KEY_CONTRACT_ID,
        &introduction.contract_id,
    );

    // Write participant list
    write_field(
        BASE_KEY_METADATA,
        META_SUB_KEY_PARTICIPANTS,
        &introduction.participants,
    );

    // Write roles list
    write_field(BASE_KEY_METADATA, META_SUB_KEY_ROLES, &introduction.roles);

    // Write sink list
    write_field(BASE_KEY_METADATA, META_SUB_KEY_SINKS, &introduction.sinks);

    // Write description
    write_field(BASE_KEY_METADATA, META_SUB_KEY_DESC, &introduction.desc);

    // Write meta
    write_field(BASE_KEY_METADATA, META_SUB_KEY_META, &introduction.meta);

    // Write initial state
    //
    // TODO: I am not sure, if a serde_json::Value can be encoded with postcard !
    // -> Two options:
    // 1. Store the bytes of the json with postcard
    // 2. Store the initial state outside of this function, using the "real" model
    //
    // I kind of tend to option 1
    write_field(
        BASE_KEY_METADATA,
        META_SUB_KEY_INIT_STATE,
        &introduction.initial_state,
    );
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

    pub fn toc() -> Duration {
        let dur = unsafe { abi::toc() };
        Duration::from_nanos(dur)
    }

    pub fn rand(min: u64, max: u64) -> u64 {
        unsafe { abi::rand(min, max) }
    }
}
