//! # Borderless ID-Types
//!
//! This library contains various types that are used as IDs for different things in the borderless ecosystem.

use core::fmt;
use std::fmt::Display;

use borderless_hash::Hash256;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use serde::{Deserialize, Serialize};

pub use uuid::Uuid;

/// Generic macro to define wrapper types around u128 and uuid.
macro_rules! impl_uuid {
    ($type:ident, $prefix_upper:literal, $prefix_lower:literal) => {
        impl $type {
            #[cfg(any(all(feature = "generate_ids", not(target_arch = "wasm32")), test))]
            pub fn generate() -> Self {
                // Start by generating a v4 uuid
                let uuid = uuid::Uuid::new_v4();
                // Then convert it to a valid $type id
                uuid.into()
            }

            pub fn into_uuid(self) -> uuid::Uuid {
                self.0
            }

            pub fn from_bytes(bytes: [u8; 16]) -> Self {
                $type(uuid::Uuid::new_v8(bytes))
            }

            pub fn into_bytes(self) -> [u8; 16] {
                self.0.into_bytes()
            }

            pub fn as_bytes(&self) -> &[u8; 16] {
                self.0.as_bytes()
            }

            pub fn parse_str(s: &str) -> Result<Self, uuid::Error> {
                let uuid = Uuid::parse_str(s)?;
                Ok(uuid.into())
            }
        }

        impl From<u128> for $type {
            fn from(value: u128) -> Self {
                let mut bytes = value.to_be_bytes();
                bytes[0] = bytes[0] & $prefix_upper | $prefix_lower;
                $type(uuid::Uuid::new_v8(bytes))
            }
        }

        impl From<uuid::Uuid> for $type {
            fn from(value: uuid::Uuid) -> Self {
                let mut bytes = value.into_bytes();
                // enforce a v8 uuid here (maybe just use new_v8 here, so that this work is delegated)
                // -> Find out, how we set the first 4 bit easily in this macro by specifying 0xcF and 0xc0
                bytes[0] = bytes[0] & $prefix_upper | $prefix_lower;
                $type(uuid::Uuid::new_v8(bytes))
            }
        }

        impl From<$type> for uuid::Uuid {
            fn from(value: $type) -> uuid::Uuid {
                value.0
            }
        }

        impl From<$type> for u128 {
            fn from(value: $type) -> u128 {
                value.0.as_u128()
            }
        }

        impl TryFrom<&str> for $type {
            type Error = uuid::Error;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                uuid::Uuid::parse_str(&value).map(Into::into)
            }
        }

        impl TryFrom<String> for $type {
            type Error = uuid::Error;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                $type::try_from(value.as_str())
            }
        }

        impl std::str::FromStr for $type {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                uuid::Uuid::parse_str(s).map(Into::into)
            }
        }

        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<$type> for String {
            fn from(value: $type) -> String {
                value.0.to_string()
            }
        }

        impl From<&$type> for String {
            fn from(value: &$type) -> String {
                value.0.to_string()
            }
        }

        impl AsRef<[u8]> for $type {
            fn as_ref(&self) -> &[u8] {
                self.0.as_bytes()
            }
        }

        impl AsRef<[u8; 16]> for $type {
            fn as_ref(&self) -> &[u8; 16] {
                self.0.as_bytes()
            }
        }
    };
}

/// Generic macro to define wrapper types around u128 and uuid.
macro_rules! impl_u32id {
    ($type:ident) => {
        impl From<u32> for $type {
            fn from(value: u32) -> Self {
                $type(value)
            }
        }

        impl $type {
            pub const fn from_const(value: u32) -> $type {
                $type(value)
            }
        }

        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

/// An agent-id used to identify software-agents in the borderless-ecosystem.
///
/// These ids are version 8 [uuids](https://en.wikipedia.org/wiki/Universally_unique_identifier), where
/// the first four bits of the uuid are set to `0xa`, to indicate that it is an agent-id and not another uuid based id.
///
/// Example:
/// ```sh
/// afc23cb3-f447-8107-8f93-9bfb8e1d157d
/// ```
///
/// All uuid-based ids used in the borderless-ecosystem have a different prefix, based on what the id is used for.
/// This mechanism ensures that you cannot mistake an agent-id for e.g. a contract-id and vice versa. Even if you convert the agent-id
/// back into a uuid and the result into a contract-id, the results are different.
///
/// The implementation of the IDs is compliant with [RFC9562](https://www.ietf.org/rfc/rfc9562.html#name-uuid-version-8),
/// as we utilize standard version 8 uuids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct AgentId(uuid::Uuid);
impl_uuid!(AgentId, 0xaf, 0xa0);

/// The borderless-id used to identify participants in the borderless-network.
///
/// These ids are version 8 [uuids](https://en.wikipedia.org/wiki/Universally_unique_identifier), where
/// the first four bits of the uuid are set to `0xb`, to indicate that it is a borderless-id and not another uuid based id.
///
/// Example:
/// ```sh
/// 0bc23cb3-f447-8107-8f93-9bfb8e1d157d
/// ```
///
/// All uuid-based ids used in the borderless-ecosystem have a different prefix, based on what the id is used for.
/// This mechanism ensures that you cannot mistake a participant-id for e.g. a contract-id and vice versa. Even if you convert the participant-id
/// back into a uuid and the result into a contract-id, the results are different.
///
/// The implementation of the IDs is compliant with [RFC9562](https://www.ietf.org/rfc/rfc9562.html#name-uuid-version-8),
/// as we utilize standard version 8 uuids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct BorderlessId(uuid::Uuid);
impl_uuid!(BorderlessId, 0xbf, 0xb0);

/// A contract-id used to itentify different SmartContracts in the borderless-ecosystem.
///
/// These ids are version 8 [uuids](https://en.wikipedia.org/wiki/Universally_unique_identifier), where
/// the first four bits of the uuid are set to `0xc`, to indicate that it is a contract-id and not another uuid based id.
///
/// Example:
/// ```sh
/// cfc23cb3-f447-8107-8f93-9bfb8e1d157d
/// ```
///
/// All uuid-based ids used in the borderless-ecosystem have a different prefix, based on what the id is used for.
/// This mechanism ensures that you cannot mistake a contract-id for e.g. a process-id and vice versa. Even if you convert the contract-id
/// back into a uuid and the result into a process-id, the results are different.
///
/// The implementation of the IDs is compliant with [RFC9562](https://www.ietf.org/rfc/rfc9562.html#name-uuid-version-8),
/// as we utilize standard version 8 uuids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct ContractId(uuid::Uuid);
impl_uuid!(ContractId, 0xcf, 0xc0);

/// A decentralized-id used to itentify different assets and documents in the borderless-ecosystem.
///
/// These ids are version 8 [uuids](https://en.wikipedia.org/wiki/Universally_unique_identifier), where
/// the first four bits of the uuid are set to `0xd`, to indicate that it is a decentralized-id and not another uuid based id.
///
/// Example:
/// ```sh
/// dfc23cb3-f447-8107-8f93-9bfb8e1d157d
/// ```
///
/// All uuid-based ids used in the borderless-ecosystem have a different prefix, based on what the id is used for.
/// This mechanism ensures that you cannot mistake a contract-id for e.g. a process-id and vice versa. Even if you convert the contract-id
/// back into a uuid and the result into a process-id, the results are different.
///
/// The implementation of the IDs is compliant with [RFC9562](https://www.ietf.org/rfc/rfc9562.html#name-uuid-version-8),
/// as we utilize standard version 8 uuids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct Did(uuid::Uuid);
impl_uuid!(Did, 0xdf, 0xd0);

/// A role-id.
///
/// Wraps an u32 to indicate its usage as a role-identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleId(u32);
impl_u32id!(RoleId);

/// An action-id.
///
/// Wraps an u32 to indicate its usage as a action-identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionId(u32);
impl_u32id!(ActionId);

/// Type used to identify blocks.
///
/// In principle a block is uniquely defined by its hash, however it may be useful to know where to find the block.
/// A `BlockIdentifier` adds the chain-id and block-number to the hash, so we can easily lookup a block based on its identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct BlockIdentifier {
    /// Chain-ID of the block.
    pub chain_id: u32,
    /// Number of the block.
    pub number: u64,
    /// Hash of the block.
    ///
    /// Note: The hash of a block is not the hash over the serialized data.
    /// It is the sum of three hashes: `Hash(Header) + MerkleRoot(Txs) + MerkleRoot(SignatureWithKeys)`
    pub hash: Hash256,
}

impl BlockIdentifier {
    /// Constructs a new `BlockIdentifier` from its raw parts
    pub fn new(chain_id: u32, number: u64, hash: Hash256) -> Self {
        Self {
            chain_id,
            number,
            hash,
        }
    }

    /// Decodes a `BlockIdentifier` from bytes
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, InvalidBlockIdentifier> {
        if bytes.len() != ENCODED_BLOCK_ID_LEN {
            Err(InvalidBlockIdentifier(bytes.len()))
        } else {
            let mut bytes = Bytes::from(bytes);
            let chain_id = bytes.get_u32();
            let number = bytes.get_u64();
            let mut hash_slice = [0u8; 32];
            bytes.copy_to_slice(&mut hash_slice);
            Ok(BlockIdentifier {
                chain_id,
                number,
                hash: hash_slice.into(),
            })
        }
    }

    /// Encodes a `BlockIdentifier` to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = BytesMut::with_capacity(std::mem::size_of::<BlockIdentifier>());
        buf.put_u32(self.chain_id);
        buf.put_u64(self.number);
        buf.put_slice(self.hash.as_ref());
        buf.into()
    }
}

/// Lenght of an encoded TxIdentifier
///
/// 4 bytes for the chain-id (u32)
/// 8 bytes for the number (u64)
/// 32 bytes for the hash (which is basically [u8; 32])
const ENCODED_BLOCK_ID_LEN: usize = 4 + 8 + 32;

#[derive(Debug, PartialEq)]
pub struct InvalidBlockIdentifier(pub usize);
impl Display for InvalidBlockIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed to decode block identifier - expected {ENCODED_BLOCK_ID_LEN} bytes, got {}",
            self.0
        )
    }
}
impl std::error::Error for InvalidBlockIdentifier {}

// This is necessary for the update functionality.
// We need a lexicographical sort of the incoming block-ids,
// sorted by their hash.
impl PartialOrd for BlockIdentifier {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BlockIdentifier {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hash.cmp(&other.hash)
    }
}

impl Display for BlockIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.chain_id, self.number, self.hash)
    }
}

/// Type used to identify transactions.
///
/// In principal a transaction is uniquely defined by its hash, however it may be useful to know where to find the transaction.
/// An identifier adds the chain-id and block-number to the hash, so we can easily lookup a transaction based on its identifier.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxIdentifier {
    /// Chain-ID of the transaction.
    pub chain_id: u32,
    /// Block-Number of the block that holds the transaction.
    pub number: u64,
    /// Hash of the transaction.
    pub hash: Hash256,
}

impl TxIdentifier {
    /// Constructs a new [`TxIdentifier`]
    pub fn new(chain_id: u32, number: u64, hash: Hash256) -> Self {
        TxIdentifier {
            chain_id,
            number,
            hash,
        }
    }

    /// Decodes a `TxIdentifier` from bytes
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, InvalidTxIdentifier> {
        if bytes.len() != ENCODED_TX_ID_LEN {
            Err(InvalidTxIdentifier(bytes.len()))
        } else {
            let mut bytes = Bytes::from(bytes);
            let chain_id = bytes.get_u32();
            let number = bytes.get_u64();
            let mut hash_slice = [0u8; 32];
            bytes.copy_to_slice(&mut hash_slice);
            Ok(TxIdentifier {
                chain_id,
                number,
                hash: hash_slice.into(),
            })
        }
    }

    /// Encodes a `TxIdentifier` as bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = BytesMut::with_capacity(std::mem::size_of::<TxIdentifier>());
        buf.put_u32(self.chain_id);
        buf.put_u64(self.number);
        buf.put_slice(self.hash.as_ref());
        buf.into()
    }
}

/// Lenght of an encoded TxIdentifier
///
/// 4 bytes for the chain-id (u32)
/// 8 bytes for the number (u64)
/// 32 bytes for the hash (which is basically [u8; 32])
const ENCODED_TX_ID_LEN: usize = 4 + 8 + 32;

#[derive(Debug, PartialEq)]
pub struct InvalidTxIdentifier(pub usize);
impl Display for InvalidTxIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed to decode transaction identifier - expected {ENCODED_TX_ID_LEN} bytes, got {}",
            self.0
        )
    }
}
impl std::error::Error for InvalidTxIdentifier {}

impl Display for TxIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.chain_id, self.number, self.hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn agent_id_prefix() {
        for _ in 0..1_000_000 {
            let base_id = Uuid::new_v4();
            let process_id = AgentId::from(base_id);
            let pid_string = process_id.to_string();
            assert_eq!(
                pid_string.chars().next(),
                Some('a'),
                "Process-IDs must be prefixed with 'a' in string representation"
            );
            let back_to_uuid: Uuid = process_id.into();
            assert_ne!(base_id, back_to_uuid);
            // Check, if first four bits match 'a'
            // NOTE: Bit-level-hacking here: bits abcdefgh & 11110000 = abcd0000
            // -> so i can match on byte level agains abcd0000 (in this case 0xb0)
            assert_eq!(back_to_uuid.as_bytes()[0] & 0xF0, 0xa0);
        }
    }

    #[test]
    fn borderless_id_prefix() {
        for _ in 0..1_000_000 {
            let base_id = Uuid::new_v4();
            let borderless_id = BorderlessId::from(base_id);
            let pid_string = borderless_id.to_string();
            assert_eq!(
                pid_string.chars().next(),
                Some('b'),
                "Borderless-IDs must be prefixed with 'b' in string representation"
            );
            let back_to_uuid: Uuid = borderless_id.into();
            assert_ne!(base_id, back_to_uuid);
            // Check, if first four bits match 'b'
            // NOTE: Bit-level-hacking here: bits abcdefgh & 11110000 = abcd0000
            // -> so i can match on byte level agains abcd0000 (in this case 0x00)
            assert_eq!(back_to_uuid.as_bytes()[0] & 0xF0, 0xb0);
        }
    }

    #[test]
    fn contract_id_prefix() {
        for _ in 0..1_000_000 {
            let base_id = Uuid::new_v4();
            let contract_id = ContractId::from(base_id);
            let cid_string = contract_id.to_string();
            assert_eq!(
                cid_string.chars().next(),
                Some('c'),
                "Contract-IDs must be prefixed with 'c' in string representation"
            );
            let back_to_uuid: Uuid = contract_id.into();
            assert_ne!(base_id, back_to_uuid);
            // Check, if first four bits match 'c'
            // NOTE: Bit-level-hacking here: bits abcdefgh & 11110000 = abcd0000
            // -> so i can match on byte level agains abcd0000 (in this case 0xc0)
            assert_eq!(back_to_uuid.as_bytes()[0] & 0xF0, 0xc0);
        }
    }

    #[test]
    fn did_prefix() {
        for _ in 0..1_000_000 {
            let base_id = Uuid::new_v4();
            let asset_id = Did::from(base_id);
            let aid_string = asset_id.to_string();
            assert_eq!(
                aid_string.chars().next(),
                Some('d'),
                "Decentralized-IDs must be prefixed with 'd' in string representation"
            );
            let back_to_uuid: Uuid = asset_id.into();
            assert_ne!(base_id, back_to_uuid);
            // Check, if first four bits match 'd'
            // NOTE: Bit-level-hacking here: bits abcdefgh & 11110000 = abcd0000
            // -> so i can match on byte level agains abcd0000 (in this case 0xa0)
            assert_eq!(back_to_uuid.as_bytes()[0] & 0xF0, 0xd0);
        }
    }

    #[test]
    fn differentiate_id_types() {
        // This test ensures, that all ID types generate different bit-level representations.
        // In other words: They do not match, even if I use the same uuid to generate them.
        // This allows us to easily spot, which ID type we have and prevents cross-matches.
        for _ in 0..1_000_000 {
            let base_id = Uuid::new_v4();
            let participant_id: Uuid = BorderlessId::from(base_id).into();
            let asset_id: Uuid = Did::from(base_id).into();
            let process_id: Uuid = AgentId::from(base_id).into();
            let contract_id: Uuid = ContractId::from(base_id).into();
            // NOTE: Check all permutations, just to be sure:
            assert_ne!(base_id, participant_id);
            assert_ne!(base_id, asset_id);
            assert_ne!(base_id, process_id);
            assert_ne!(base_id, contract_id);
            assert_ne!(participant_id, asset_id);
            assert_ne!(participant_id, process_id);
            assert_ne!(participant_id, contract_id);
            assert_ne!(asset_id, process_id);
            assert_ne!(asset_id, contract_id);
            assert_ne!(process_id, contract_id);
        }
    }

    #[test]
    fn agent_id_construction() {
        for _ in 0..1_000_000 {
            let base_id = Uuid::new_v4();
            let base_u128 = base_id.as_u128();
            let from_uuid = AgentId::from(base_id);
            let from_u128 = AgentId::from(base_u128);
            assert_eq!(from_uuid, from_u128);
            let back_to_uuid: Uuid = from_uuid.into();
            let back_to_u128: u128 = from_u128.into();
            assert_eq!(back_to_uuid, Uuid::from_u128(back_to_u128));
            assert_eq!(back_to_uuid.as_u128(), back_to_u128); // this is redundant - but let's stay paranoid
            assert_ne!(base_id, back_to_uuid);
            assert_ne!(base_id.as_u128(), back_to_u128);
        }
    }

    #[test]
    fn borderless_id_construction() {
        for _ in 0..1_000_000 {
            let base_id = Uuid::new_v4();
            let base_u128 = base_id.as_u128();
            let from_uuid = BorderlessId::from(base_id);
            let from_u128 = BorderlessId::from(base_u128);
            assert_eq!(from_uuid, from_u128);
            let back_to_uuid: Uuid = from_uuid.into();
            let back_to_u128: u128 = from_u128.into();
            assert_eq!(back_to_uuid, Uuid::from_u128(back_to_u128));
            assert_eq!(back_to_uuid.as_u128(), back_to_u128); // this is redundant - but let's stay paranoid
            assert_ne!(base_id, back_to_uuid);
            assert_ne!(base_id.as_u128(), back_to_u128);
        }
    }

    #[test]
    fn contract_id_construction() {
        for _ in 0..1_000_000 {
            let base_id = Uuid::new_v4();
            let base_u128 = base_id.as_u128();
            let from_uuid = ContractId::from(base_id);
            let from_u128 = ContractId::from(base_u128);
            assert_eq!(from_uuid, from_u128);
            let back_to_uuid: Uuid = from_uuid.into();
            let back_to_u128: u128 = from_u128.into();
            assert_eq!(back_to_uuid, Uuid::from_u128(back_to_u128));
            assert_eq!(back_to_uuid.as_u128(), back_to_u128); // this is redundant - but let's stay paranoid
            assert_ne!(base_id, back_to_uuid);
            assert_ne!(base_id.as_u128(), back_to_u128);
        }
    }

    #[test]
    fn did_construction() {
        for _ in 0..1_000_000 {
            let base_id = Uuid::new_v4();
            let base_u128 = base_id.as_u128();
            let from_uuid = Did::from(base_id);
            let from_u128 = Did::from(base_u128);
            assert_eq!(from_uuid, from_u128);
            let back_to_uuid: Uuid = from_uuid.into();
            let back_to_u128: u128 = from_u128.into();
            assert_eq!(back_to_uuid, Uuid::from_u128(back_to_u128));
            assert_eq!(back_to_uuid.as_u128(), back_to_u128); // this is redundant - but let's stay paranoid
            assert_ne!(base_id, back_to_uuid);
            assert_ne!(base_id.as_u128(), back_to_u128);
        }
    }
}
