//! Definitions of special registers
//!
//! We use registers to share data between host and guest.
//! Each register is defined by a unique register-id (which is a `u64`) and can hold an arbitrary amount of bytes.
//!
//! It is generally assumed, that the guest "knows" how to handle the bytes of the register.

/// Register used for atomic operations
#[allow(dead_code)]
pub(crate) const REGISTER_ATOMIC_OP: u64 = u64::MAX - 1;

/// Register used to feed input into the contract
pub const REGISTER_INPUT: u64 = 0;

/// Register used to return output back to the caller
pub const REGISTER_OUTPUT: u64 = 1;

/// Register used to feed the writer-id into the contract
pub const REGISTER_WRITER: u64 = 2;

/// Register used to feed the executor-id into the contract
///
/// Should only be accessed by the macro and never by the user, since this could introduce side-effects
pub const REGISTER_EXECUTOR: u64 = 3;

/// Register used to feed the transaction context (tx-id + tx-index) into the contract
pub const REGISTER_TX_CTX: u64 = 4;

/// Register used to feed the block context (block-id + block-timestamp) into the contract
pub const REGISTER_BLOCK_CTX: u64 = 5;

// --- State and action http requests
/// Register to feed http requests into the contract
pub const REGISTER_INPUT_HTTP_PATH: u64 = 1024;

/// Register to feed http request payloads into the contract
pub const REGISTER_INPUT_HTTP_PAYLOAD: u64 = 1025;

/// Register used to write the output http-status (as big-endian u16)
pub const REGISTER_OUTPUT_HTTP_STATUS: u64 = 2048;

/// Register used to write the output http payload
pub const REGISTER_OUTPUT_HTTP_RESULT: u64 = 2049;

// --- SW-Agent related registers

/// Contains the head (everything except the body) of an http-request
pub const REGISTER_REQUEST_HEAD: u64 = 4096;

/// Contains the body of an http-request
pub const REGISTER_REQUEST_BODY: u64 = 4097;

/// Contains the head (everything except the body) of an http-request
pub const REGISTER_RESPONSE_HEAD: u64 = 4098;

/// Contains the body of an http-request
pub const REGISTER_RESPONSE_BODY: u64 = 4099;

// Cursor content start from this register
pub const REGISTER_CURSOR: u64 = 2 << 32;
