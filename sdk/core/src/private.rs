// We have to set the path explicitly, because the module is named "__private", while the directory is named "private"
#[path = "private/registers.rs"]
pub mod registers;
#[path = "private/storage_keys.rs"]
pub mod storage_keys;
#[path = "private/storage_traits.rs"]
pub mod storage_traits;

#[path = "private/env.rs"]
pub mod env;

use borderless_abi as abi;

use registers::*;
use serde::{de::DeserializeOwned, Serialize};

use crate::error;
use crate::prelude::ledger::LedgerEntry;
use crate::prelude::{Id, Topic};
// --- PLAYGROUND FOR NEW ABI STUFF

#[allow(unused_variables)]
pub fn send_http_rq(
    rq_head: impl AsRef<str>,
    rq_body: impl AsRef<[u8]>,
) -> Result<(String, Vec<u8>), String> {
    #[cfg(target_arch = "wasm32")]
    {
        write_string_to_register(REGISTER_REQUEST_HEAD, rq_head);
        write_register(REGISTER_REQUEST_BODY, &rq_body);

        // TODO: We also have to provide a mock implementation for this,
        // otherwise the testing would not work
        let _result = {
            unsafe {
                abi::send_http_rq(
                    REGISTER_REQUEST_HEAD,
                    REGISTER_REQUEST_BODY,
                    REGISTER_RESPONSE_HEAD,
                    REGISTER_RESPONSE_BODY,
                    REGISTER_ATOMIC_OP,
                )
            }
        };

        match _result {
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        panic!("http related functionality is only available from within wasm code")
    }
}

/// Simple wrapper to send websocket messages via the abi
///
/// Requires that everything is setup for the websocket, otherwise this will always fail.
#[allow(unused_variables)]
pub fn send_ws_msg(msg: impl AsRef<[u8]>) -> anyhow::Result<()> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        match abi::send_ws_msg(msg.as_ref().as_ptr() as _, msg.as_ref().len() as _) {
            0 => Ok(()),
            _ => Err(anyhow::Error::msg("failed to send websocket message")),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        panic!("http related functionality is only available from within wasm code")
    }
}

/// Helper function that marks a register value as "required" - meaning that in case the value is not present,
/// we abort the execution (this is an implementation error of the runtime, and not an error that the user should handle).
///
/// To avoid "being in the dark" when debugging this, we log an error message before aborting - just like we do with [`read_field`].
#[allow(dead_code)]
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
    #[cfg(target_arch = "wasm32")]
    {
        env::on_chain::print(level, msg)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        env::off_chain::print(level, msg)
    }
}

pub fn read_register(register_id: u64) -> Option<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    {
        env::on_chain::read_register(register_id)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        env::off_chain::read_register(register_id)
    }
}

pub fn read_string_from_register(register_id: u64) -> Option<String> {
    read_register(register_id).and_then(|bytes| String::from_utf8(bytes).ok())
}

pub fn write_register(register_id: u64, data: impl AsRef<[u8]>) {
    #[cfg(target_arch = "wasm32")]
    {
        env::on_chain::write_register(register_id, data)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        env::off_chain::write_register(register_id, data)
    }
}

pub fn write_string_to_register(register_id: u64, string: impl AsRef<str>) {
    write_register(register_id, string.as_ref());
}

fn storage_write(base_key: u64, sub_key: u64, value: impl AsRef<[u8]>) {
    let value = value.as_ref();

    #[cfg(target_arch = "wasm32")]
    {
        env::on_chain::storage_write(base_key, sub_key, value)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        env::off_chain::storage_write(base_key, sub_key, value)
    }
}

fn storage_read(base_key: u64, sub_key: u64) -> Option<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    {
        env::on_chain::storage_read(base_key, sub_key)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        env::off_chain::storage_read(base_key, sub_key)
    }
}

pub fn storage_remove(base_key: u64, sub_key: u64) {
    #[cfg(target_arch = "wasm32")]
    {
        env::on_chain::storage_remove(base_key, sub_key)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        env::off_chain::storage_remove(base_key, sub_key)
    }
}

pub fn storage_cursor(base_key: u64) -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        env::on_chain::storage_cursor(base_key)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        env::off_chain::storage_cursor(base_key)
    }
}

pub fn storage_has_key(base_key: u64, sub_key: u64) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        env::on_chain::storage_has_key(base_key, sub_key)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        env::off_chain::storage_has_key(base_key, sub_key)
    }
}

pub fn storage_gen_sub_key() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        env::on_chain::storage_gen_sub_key()
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        env::off_chain::storage_gen_sub_key()
    }
}

/// Reads a value from the storage via the register.
///
/// Returns `None` if no value could be found at the given storage keys.
pub fn read_field<Value>(base_key: u64, sub_key: u64) -> Option<Value>
where
    Value: DeserializeOwned,
{
    storage_read(base_key, sub_key).and_then(|bytes| {
        postcard::from_bytes::<Value>(bytes.as_slice())
            .inspect_err(|e| print(abi::LogLevel::Error, format!("Deserialization error: {e}")))
            .ok()
    })
}

pub fn write_field<Value>(base_key: u64, sub_key: u64, value: &Value)
where
    Value: Serialize,
{
    match postcard::to_allocvec::<Value>(value) {
        Ok(bytes) => storage_write(base_key, sub_key, bytes),
        Err(e) => {
            error!("SYSTEM: write-field failed base-key={base_key:x} sub-key={sub_key:x}: {e}");
            abort()
        }
    }
}

pub fn abort() -> ! {
    #[cfg(target_arch = "wasm32")]
    {
        env::on_chain::abort()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        panic!();
    }
}

pub fn subscribe(topic: Topic) -> crate::Result<()> {
    #[cfg(target_arch = "wasm32")]
    {
        env::on_chain::subscribe(topic)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        env::off_chain::subscribe(topic)
    }
}

pub fn unsubscribe(publisher: Id, topic: String) -> crate::Result<()> {
    #[cfg(target_arch = "wasm32")]
    {
        env::on_chain::unsubscribe(publisher, topic)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        env::off_chain::unsubscribe(publisher, topic)
    }
}

pub fn create_ledger_entry(entry: LedgerEntry) -> crate::Result<()> {
    let _bytes = entry.to_bytes();
    #[cfg(target_arch = "wasm32")]
    {
        env::on_chain::create_ledger_entry(entry)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        env::off_chain::create_ledger_entry(entry)
    }
}

pub mod dev {
    use crate::__private::env;
    use std::time::Duration;

    pub fn tic() {
        #[cfg(target_arch = "wasm32")]
        {
            env::on_chain::tic()
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            env::off_chain::tic()
        }
    }

    pub fn toc() -> Duration {
        #[cfg(target_arch = "wasm32")]
        {
            env::on_chain::toc()
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            env::off_chain::toc()
        }
    }

    pub fn rand(min: u64, max: u64) -> u64 {
        #[cfg(target_arch = "wasm32")]
        {
            env::on_chain::rand(min, max)
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            env::off_chain::rand(min, max)
        }
    }
}
