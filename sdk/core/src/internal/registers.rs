//! Definitions of special registers
//!
//! We use registers to share data between host and guest.
//! Each register is defined by a unique register-id (which is a `u64`) and can hold an arbitrary amount of bytes.
//!
//! It is generally assumed, that the guest "knows" how to handle the bytes of the register.

/// Register used for atomic operations
pub(crate) const REGISTER_ATOMIC_OP: u64 = u64::MAX - 1;

/// Register used to feed input into the contract
pub const REGISTER_INPUT: u64 = 0;

/// Register used to feed the caller-id into the contract
pub const REGISTER_CALLER: u64 = 1;

/// Register used to feed the executor-id into the contract
pub const REGISTER_EXECUTOR: u64 = 2;

/// Register used to feed the tx-id into the contract
pub const REGISTER_TX_ID: u64 = 3;

/// Register used to feed the block-id into the contract
pub const REGISTER_BLOCK_ID: u64 = 4;
