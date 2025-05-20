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
    // TODO: clear_register ?

    // --- Control flow
    pub fn panic() -> !;
    pub fn panic_utf8(len: u64, ptr: u64) -> !;

    // --- Storage API
    pub fn storage_write(base_key: u64, sub_key: u64, value_ptr: u64, value_len: u64);
    pub fn storage_read(base_key: u64, sub_key: u64, register_id: u64);
    pub fn storage_remove(base_key: u64, sub_key: u64);
    pub fn storage_cursor(base_key: u64) -> u64;

    // --- Dangerous API (introduces side-effects)
    pub fn storage_has_key(base_key: u64, sub_key: u64) -> u64;
    pub fn storage_gen_sub_key() -> u64;
    pub fn storage_next_subkey(base_key: u64, from_sub_key: u64) -> u64;
    pub fn storage_query_subkey_range(base_key: u64, sub_key_start: u64, sub_key_end: u64) -> u64;

    // Profiling
    pub fn tic(); // matlab style
    pub fn toc() -> u64; // << TODO: Let's not do that, but instead print out the result, so we don't create side-effects

    // Testing
    pub fn rand(min: u64, max: u64) -> u64;

    // --- SW-Agent API ( async host functions ? )
    //
    // Let's brainstorm a little bit, what is going on here.

    // Sends a http-request to some remote entity and returns the result
    //
    // TODO: This is a weird design; usually you give pointers to wasm for reading
    pub fn send_http_rq(
        register_rq_head: u64,
        register_rq_body: u64,
        register_rs_head: u64,
        register_rs_body: u64,
        register_failure: u64,
    ) -> u64;

    // Returns the current timestamp as milliseconds since epoch
    pub fn timestamp() -> i64;

    // Send a (string) message via websocket
    pub fn send_ws_msg(msg_ptr: u64, msg_len: u64) -> u64;

    // Create a new schedule that should be called regularly
    pub fn register_schedule();

    // Open a websocket connection and register a message-hook
    //
    // -> NOTE: This may require some more complex interaction,
    // like fetching something via API first, then opening the websocket or so.
    // Maybe we can avoid this by offloading the work to the implementor, so that e.g.
    // a special function is called in case of failure, and the retry / reopen has to be implemented manually.
    pub fn open_ws_connection();

    // Sends a batch of http-requests to some remote entities in parallel and returns the results
    pub fn batch_http_rqs();

    // Maybe something that executes, everytime a contract finished its execution ?
    pub fn register_hook();
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
