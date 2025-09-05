use crate::__private::{LedgerEntry, REGISTER_ATOMIC_OP};
use crate::error;
use crate::prelude::Topic;
use borderless_abi as abi;
use std::time::Duration;
// The on_chain environment.

pub fn print(level: abi::LogLevel, msg: impl AsRef<str>) {
    unsafe {
        abi::print(
            msg.as_ref().as_ptr() as _,
            msg.as_ref().len() as _,
            level as u32,
        );
    }
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
                error!("SYSTEM: invalid return code in 'storage_has_key' func");
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

pub fn register_len(register_id: u64) -> Option<u64> {
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

pub fn subscribe(topic: Topic) -> crate::Result<()> {
    unsafe {
        match abi::subscribe(
            topic.publisher.as_ref().as_ptr() as _,
            topic.topic.as_ptr() as _,
            topic.topic.len() as _,
            topic.method.as_ptr() as _,
            topic.method.len() as _,
        ) {
            0 => Ok(()),
            1 => Err(crate::Error::msg(
                "subscriptions are only relevant to agents",
            )),
        }
    }
}

pub fn create_ledger_entry(entry: LedgerEntry) -> crate::Result<()> {
    let bytes = entry.to_bytes()?;
    unsafe {
        match abi::create_ledger_entry(bytes.as_ptr() as _, bytes.len() as _) {
            0 => Ok(()),
            1 => Err(crate::Error::msg("creditor not in participants")),
            2 => Err(crate::Error::msg("debitor not in participants")),
            3 => Err(crate::Error::msg(
                "creditor and debitor not in participants",
            )),
            _ => Err(crate::Error::msg("failed to create ledger entry")),
        }
    }
}

pub fn abort() -> ! {
    core::arch::wasm32::unreachable()
}

pub fn tic() {
    unsafe { abi::tic() }
}

// TODO: Change this function to not produce side-effects
pub fn toc() -> Duration {
    let dur = unsafe { abi::toc() };
    Duration::from_nanos(dur)
}

// TODO: Remove this
pub fn rand(min: u64, max: u64) -> u64 {
    unsafe { abi::rand(min, max) }
}
