//! Definition of the wasm interface
//!
//! This library does not implement the interface by itself.

#![no_std]
extern "C" {
    pub fn print(ptr: u64, len: u64, level: u32);

    // --- Register functions
    pub fn read_register(register_id: u64, wasm_ptr: u64);
    pub fn write_register(register_id: u64, wasm_ptr: u64, wasm_ptr_len: u64);
    pub fn register_len(register_id: u64) -> u64;

    // --- Control flow
    pub fn panic() -> !;
    pub fn panic_utf8(len: u64, ptr: u64) -> !;

    // --- Storage API ( Basic )
    pub fn storage_write(base_key: u64, sub_key: u64, value_ptr: u64, value_len: u64);
    pub fn storage_read(base_key: u64, sub_key: u64, register_id: u64);
    pub fn storage_remove(base_key: u64, sub_key: u64);
    pub fn storage_cursor(base_key: u64) -> u64;
    pub fn storage_has_key(base_key: u64, sub_key: u64) -> u64;

    // --- Storage API ( Advanced )
    pub fn storage_gen_sub_key() -> u64; // WARNING: Introduces side-effects ! Use with caution
    pub fn storage_next_subkey(base_key: u64, from_sub_key: u64) -> u64;
    pub fn storage_query_subkey_range(base_key: u64, sub_key_start: u64, sub_key_end: u64) -> u64;

    // --- Ledger-API
    pub fn create_ledger_entry(wasm_ptr: u64, wasm_len: u64) -> u64;

    // Profiling
    pub fn tic(); // matlab style
    pub fn toc() -> u64; // << TODO: Let's not do that, but instead print out the result, so we don't create side-effects

    // Testing
    pub fn rand(min: u64, max: u64) -> u64;

    // --- SW-Agent API

    // Sends a http-request to some remote entity and returns the result
    //
    // NOTE: We use registers here instead of pointers to make our live easier,
    // because we essentially return multiple values from this function with unknown size.
    pub fn send_http_rq(
        register_rq_head: u64,
        register_rq_body: u64,
        register_rs_head: u64,
        register_rs_body: u64,
        register_failure: u64,
    ) -> u64;

    // Returns the current timestamp as milliseconds since epoch
    pub fn timestamp() -> i64;

    // Send a message via websocket
    pub fn send_ws_msg(msg_ptr: u64, msg_len: u64) -> u64;

    pub fn subscribe(id_ptr: u64, topic_ptr: u64, topic_len: u64) -> u64;

    pub fn unsubscribe(id_ptr: u64, topic_ptr: u64, topic_len: u64) -> u64;
}

#[derive(Debug)]
#[repr(u32)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}
