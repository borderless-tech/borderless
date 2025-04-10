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

/// Register used to feed the writer-id into the contract
pub const REGISTER_WRITER: u64 = 1;

// NOTE: I think this is not a good idea - the executor differs, depending on where the contract is executed.
// Giving users access to the executor-id enables them to write logic, that breaks the distributed contract execution !
//
// /// Register used to feed the executor-id into the contract
// pub const REGISTER_EXECUTOR: u64 = 2;

/// Register used to feed the transaction context (tx-id + tx-index) into the contract
pub const REGISTER_TX_CTX: u64 = 3;

/// Register used to feed the block context (block-id + block-timestamp) into the contract
pub const REGISTER_BLOCK_CTX: u64 = 4;
