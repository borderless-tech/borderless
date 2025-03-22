pub mod http;
pub mod log;
pub mod registers;
pub mod storage;

pub use anyhow::Result;

// Forward the entire serde-json module
pub use serde_json as json;

use registers::REGISTER_ATOMIC_OP;
use serde::{de::DeserializeOwned, Serialize};

/// A contract-id
// TODO: Use real sdk type here
pub struct ContractId([u8; 16]);

impl ContractId {
    /// Creates an "empty" contract-id (just for test purposes)
    pub fn empty() -> Self {
        Self::from(0u128)
    }

    pub fn new(id: u128) -> Self {
        Self::from(id)
    }

    /// Get the underlying bytes
    pub fn to_bytes(&self) -> [u8; 16] {
        self.0
    }
}

impl AsRef<[u8]> for ContractId {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<u128> for ContractId {
    fn from(value: u128) -> Self {
        let mut bytes = value.to_be_bytes();
        bytes[0] = bytes[0] & 0xcF | 0xc0;
        Self(bytes)
    }
}

pub mod error {
    pub use anyhow::{Context, Error};
}
