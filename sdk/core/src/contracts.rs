use std::fmt::Display;

use borderless_hash::Hash256;
use borderless_id_types::{BlockIdentifier, TxIdentifier};
use serde::{Deserialize, Serialize};

use crate::{events::Sink, BorderlessId, ContractId};

/// Contract Environment
pub mod env;

pub mod ledger;

/// Maps a `BorderlessId` to a role in a smart-contract
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    /// Borderless-ID of the contract-participant that we assign the role to
    pub participant_id: BorderlessId,

    /// Alias of the role that is assigned
    ///
    /// Roles are defined as enums and usually represented as strings to the outside world.
    pub role: String,
}

/// Contract-Info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub contract_id: ContractId,
    pub participants: Vec<BorderlessId>,
    pub roles: Vec<Role>,
    pub sinks: Vec<Sink>,
}

/// Transaction-Context
///
/// Combines the [`TxIdentifier`] with the index of the transaction inside the block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxCtx {
    pub tx_id: TxIdentifier,
    pub index: u64,
}

impl TxCtx {
    /// Creates a dummy `TxCtx` without meaning.
    ///
    /// Useful for testing.
    pub fn dummy() -> Self {
        Self {
            tx_id: TxIdentifier::new(999, 999, Hash256::empty()),
            index: 0,
        }
    }

    /// Use postcard to encode the `TxCtx`
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Use postcard to decode the `TxCtx`
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

impl Display for TxCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tx-ID: {}, index: {}", self.tx_id, self.index)
    }
}

/// Block-Context
///
/// Combines the [`BlockIdentifier`] with the timestamp of the block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockCtx {
    pub block_id: BlockIdentifier,
    pub timestamp: u64,
}

impl BlockCtx {
    /// Use postcard to encode the `BlockCtx`
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Use postcard to decode the `BlockCtx`
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

impl Display for BlockCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Block-ID: {}, timestamp: {}",
            self.block_id, self.timestamp
        )
    }
}
