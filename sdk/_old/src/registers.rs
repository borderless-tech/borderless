//! Definitions of special registers
//!
//! We use registers to share data between host and guest.
//! Each register is defined by a unique register-id (which is a `u64`) and can hold an arbitrary amount of bytes.
//!
//! It is generally assumed, that the guest "knows" how to handle the bytes of the register.

/// Register used for atomic operations
pub(crate) const REGISTER_ATOMIC_OP: u64 = u64::MAX - 1;

/// Register used to input the url-path
pub const REGISTER_INPUT_URL_PATH: u64 = 2;

/// Register used to input the url-query parameters
pub const REGISTER_INPUT_URL_QUERY: u64 = 3;

/// Register used to input the writer participant-id
pub const REGISTER_INPUT_WRITER: u64 = 4;

/// Register used to input the executor's participant-id
pub const REGISTER_INPUT_EXECUTOR: u64 = 5;

pub const REGISTER_OUTPUT_HTTP_STATUS: u64 = 1024;

pub const REGISTER_OUTPUT_HTTP_RESULT: u64 = 1025;
