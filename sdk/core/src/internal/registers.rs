//! Definitions of special registers
//!
//! We use registers to share data between host and guest.
//! Each register is defined by a unique register-id (which is a `u32`) and can hold an arbitrary amount of bytes.
//!
//! It is generally assumed, that the guest "knows" how to handle the bytes of the register.

/// Register used for atomic operations
pub(crate) const REGISTER_ATOMIC_OP: u32 = u32::MAX - 1;

/// Register used to feed input into the contract
pub const REGISTER_INPUT: u32 = 0;
