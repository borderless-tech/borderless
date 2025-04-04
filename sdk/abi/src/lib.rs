//! Definition of the wasm interface
//!
//! This library does not implement the interface by itself.

#![no_std]
extern "C" {
    pub fn print(ptr: u64, len: u64, level: u64);

    // --- Register functions
    pub fn read_register(register_id: u64, wasm_ptr: u64);
    pub fn write_register(register_id: u64, wasm_ptr: u64, wasm_ptr_len: u64);
    pub fn register_len(register_id: u64) -> u64;

    // --- Control flow
    pub fn panic() -> !;
    pub fn panic_utf8(len: u64, ptr: u64) -> !;

    // --- Storage API
    pub fn storage_write(base_key: u64, sub_key: u64, value_ptr: u64, value_len: u64);
    pub fn storage_read(base_key: u64, sub_key: u64, register_id: u64);
    pub fn storage_remove(base_key: u64, sub_key: u64);
    pub fn storage_has_key(base_key: u64, sub_key: u64) -> u64;

    pub fn storage_gen_sub_key() -> u64;

    pub fn storage_begin_acid_txn() -> u64;
    pub fn storage_commit_acid_txn() -> u64;

    /*
     * state.field_1 ->    base-key = 0xf4a1,  sub-key = 0x0000
     * state.vec_1   ->    base-key = 0xa1b3,  sub-key = 0x0001 -> 0x00ff,
     *
     * storage_read ( base, sub ) -> key = [ contract-id, base-key, sub-key ]
     *                                     | ---------   256 bit ---------- |
     */

    // pub fn storage_iter_prefix(prefix_ptr: u64, prefix_len: u64) -> u64;
    // pub fn storage_iter_range(start_ptr: u64, start_len: u64, end_ptr: u64, end_len: u64) -> u64;
    // pub fn storage_iter_next(iterator_id: u64, key_register_id: u64, value_register_id: u64)
    //     -> u64;

    // --- Profiling
    pub fn tic(); // matlab style
    pub fn tocp(); // prints the output
    pub fn toc() -> u64; // returns the nanoseconds as u64

    // --- Testing
    pub fn rand(min: u64, max: u64) -> u64;
}

#[repr(u64)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}
