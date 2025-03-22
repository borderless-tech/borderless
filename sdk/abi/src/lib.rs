//! Definition of the wasm interface
//!
//! This library does not implement the interface by itself.

// TODO: Stop using u32 for booleans, as wasm is 32-bit by default -> Use u32 or i32 instead
#![no_std]
extern "C" {
    pub fn print(ptr: u32, len: u32, level: u32);

    // --- Register functions
    pub fn read_register(register_id: u32, wasm_ptr: u32);
    pub fn write_register(register_id: u32, wasm_ptr: u32, wasm_ptr_len: u32);
    pub fn register_len(register_id: u32) -> u32;

    // --- Control flow
    pub fn panic() -> !;
    pub fn panic_utf8(len: u32, ptr: u32) -> !;

    // --- Storage API
    pub fn storage_write(
        base_key: u32,
        sub_key: u32,
        value_ptr: u32,
        value_len: u32,
        register_id: u32,
    );
    pub fn storage_read(base_key: u32, sub_key: u32, register_id: u32);
    pub fn storage_remove(base_key: u32, sub_key: u32);
    pub fn storage_has_key(base_key: u32, sub_key: u32) -> u32;

    pub fn storage_random_key() -> u32;

    pub fn storage_begin_acid_txn() -> u32;
    pub fn storage_commit_acid_txn() -> u32;

    /*
     * state.field_1 ->    base-key = 0xf4a1,  sub-key = 0x0000
     * state.vec_1   ->    base-key = 0xa1b3,  sub-key = 0x0001 -> 0x00ff,
     *
     * storage_read ( base, sub ) -> key = [ contract-id, base-key, sub-key ]
     *                                     | ---------   256 bit ---------- |
     */

    // pub fn storage_iter_prefix(prefix_ptr: u32, prefix_len: u32) -> u32;
    // pub fn storage_iter_range(start_ptr: u32, start_len: u32, end_ptr: u32, end_len: u32) -> u32;
    // pub fn storage_iter_next(iterator_id: u32, key_register_id: u32, value_register_id: u32)
    //     -> u32;

    // --- Profiling
    pub fn tic(); // matlab style
    pub fn toc();

    // --- Testing
    pub fn rand(min: u32, max: u32) -> u32;
}

#[repr(u32)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}
