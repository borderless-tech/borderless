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
            /// Generates a new ID
            #[cfg(any(all(feature = "generate_ids", not(target_arch = "wasm32")), test))]
            pub fn generate() -> Self {
                // Start by generating a v4 uuid
                let uuid = uuid::Uuid::new_v4();
                // Then convert it to a valid $type id
                uuid.into()
            }

            /// Converts the ID into a [`uuid::Uuid`]
            pub fn into_uuid(self) -> uuid::Uuid {
                self.0
            }

            // NOTE: I am not sure; we have two options here. Either we just construct a uuid based on the bytes,
            // but then it may not be a valid $type. Or, we enforce our bit-pattern here, so we know, that we have a valid $type,
            // BUT this changes the roundtrip encoding (bytes -> $type -> bytes), because we modified the array here.
            pub fn from_bytes(mut bytes: [u8; 16]) -> Self {
                bytes[0] = bytes[0] & $prefix_upper | $prefix_lower;
                $type(uuid::Uuid::new_v8(bytes))
            }

            /// Returns the underlying bytes
            pub fn into_bytes(self) -> [u8; 16] {
                self.0.into_bytes()
            }

            /// Returns a reference to the underlying bytes
            pub fn as_bytes(&self) -> &[u8; 16] {
                self.0.as_bytes()
            }

            /// Parses an ID from a `&str`
            pub fn parse_str(s: &str) -> Result<Self, uuid::Error> {
                let uuid = Uuid::parse_str(s)?;
                Ok(uuid.into())
            }

            /// Merges two IDs commutatively
            ///
            /// Can be used to construct database keys.
            pub fn merge(&self, other: &Self) -> [u8; 16] {
                let mut out = [0; 16];
                for i in 0..16 {
                    out[i] = self.as_bytes()[i] ^ other.as_bytes()[i];
                }
                out
            }

            /// Merges and Compacts two IDs into a `u64`
            ///
            /// Can be used to construct database keys.
            pub fn merge_compact(&self, other: &Self) -> u64 {
                let merged = self.merge(other);
                let mut out = [0; 8];
                for i in 0..8 {
                    out[i] = merged[i] ^ merged[i + 8];
                }
                u64::from_be_bytes(out)
            }
        }

        impl<'de> serde::Deserialize<'de> for $type {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                // NOTE: We delegate to the From<uuid> here
                let uuid = uuid::Uuid::deserialize(deserializer)?;
                Ok($type::from(uuid))
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Hash, PartialOrd, Ord)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Hash, PartialOrd, Ord)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Hash, PartialOrd, Ord)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Hash, PartialOrd, Ord)]
pub struct Did(uuid::Uuid);
impl_uuid!(Did, 0xdf, 0xd0);

/// An external-id used to identify external entities, that are not in the borderless-network.
///
/// These ids are version 8 [uuids](https://en.wikipedia.org/wiki/Universally_unique_identifier), where
/// the first four bits of the uuid are set to `0xe`, to indicate that it is an external-id and not another uuid based id.
///
/// Example:
/// ```sh
/// ebc23cb3-f447-8107-8f93-9bfb8e1d157d
/// ```
///
/// All uuid-based ids used in the borderless-ecosystem have a different prefix, based on what the id is used for.
/// This mechanism ensures that you cannot mistake an participant-id for e.g. a contract-id and vice versa. Even if you convert the participant-id
/// back into a uuid and the result into a contract-id, the results are different.
///
/// The implementation of the IDs is compliant with [RFC9562](https://www.ietf.org/rfc/rfc9562.html#name-uuid-version-8),
/// as we utilize standard version 8 uuids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Hash, PartialOrd, Ord)]
pub struct ExternalId(uuid::Uuid);
impl_uuid!(ExternalId, 0xef, 0xe0);
// TODO: Add tests for external ID and prefix checks

/// A flow-id used to identify `Flows` in a contract.
///
/// These ids are version 8 [uuids](https://en.wikipedia.org/wiki/Universally_unique_identifier), where
/// the first four bits of the uuid are set to `0xf`, to indicate that it is a flow-id and not another uuid based id.
///
/// Example:
/// ```sh
/// fbc23cb3-f447-8107-8f93-9bfb8e1d157d
/// ```
///
/// All uuid-based ids used in the borderless-ecosystem have a different prefix, based on what the id is used for.
/// This mechanism ensures that you cannot mistake an participant-id for e.g. a contract-id and vice versa. Even if you convert the participant-id
/// back into a uuid and the result into a contract-id, the results are different.
///
/// The implementation of the IDs is compliant with [RFC9562](https://www.ietf.org/rfc/rfc9562.html#name-uuid-version-8),
/// as we utilize standard version 8 uuids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Hash, PartialOrd, Ord)]
pub struct FlowId(uuid::Uuid);
impl_uuid!(FlowId, 0xff, 0xf0);

/// Check weather or not an array of bytes contains the prefix of an [`AgentId`].
///
/// Useful for filtering in key-value storages, when multiple ID types are used as keys or key-prefixes.
pub fn aid_prefix(bytes: impl AsRef<[u8]>) -> bool {
    let bytes = bytes.as_ref();
    if bytes.is_empty() {
        return false;
    }
    bytes[0] | 0x0f == 0xaf
}

/// Check weather or not an array of bytes contains the prefix of a [`BorderlessId`].
///
/// Useful for filtering in key-value storages, when multiple ID types are used as keys or key-prefixes.
pub fn bid_prefix(bytes: impl AsRef<[u8]>) -> bool {
    let bytes = bytes.as_ref();
    if bytes.is_empty() {
        return false;
    }
    bytes[0] | 0x0f == 0xbf
}

/// Check weather or not an array of bytes contains the prefix of a [`ContractId`].
///
/// Useful for filtering in key-value storages, when multiple ID types are used as keys or key-prefixes.
pub fn cid_prefix(bytes: impl AsRef<[u8]>) -> bool {
    let bytes = bytes.as_ref();
    if bytes.is_empty() {
        return false;
    }
    bytes[0] | 0x0f == 0xcf
}

/// Check weather or not an array of bytes contains the prefix of a [`Did`].
///
/// Useful for filtering in key-value storages, when multiple ID types are used as keys or key-prefixes.
pub fn did_prefix(bytes: impl AsRef<[u8]>) -> bool {
    let bytes = bytes.as_ref();
    if bytes.is_empty() {
        return false;
    }
    bytes[0] | 0x0f == 0xdf
}

/// Check weather or not an array of bytes contains the prefix of an [`ExternalId`].
///
/// Useful for filtering in key-value storages, when multiple ID types are used as keys or key-prefixes.
pub fn eid_prefix(bytes: impl AsRef<[u8]>) -> bool {
    let bytes = bytes.as_ref();
    if bytes.is_empty() {
        return false;
    }
    bytes[0] | 0x0f == 0xef
}

/// Check weather or not an array of bytes contains the prefix of a [`FlowId`].
///
/// Useful for filtering in key-value storages, when multiple ID types are used as keys or key-prefixes.
pub fn fid_prefix(bytes: impl AsRef<[u8]>) -> bool {
    let bytes = bytes.as_ref();
    if bytes.is_empty() {
        return false;
    }
    bytes[0] | 0x0f == 0xff
}

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

    pub fn genesis(chain_id: u32) -> Self {
        Self::new(chain_id, 0, Hash256::empty())
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
    use serde::de::DeserializeOwned;
    use uuid::Uuid;

    #[test]
    fn tx_id_encode_decode() {
        let tx_id = TxIdentifier::new(1, 2, Hash256::empty());
        let bytes = tx_id.to_bytes();
        let result = TxIdentifier::from_bytes(bytes);
        assert!(result.is_ok(), "decoding failed: {}", result.unwrap_err());
        assert_eq!(result.unwrap(), tx_id);
    }

    #[test]
    fn tx_id_display() {
        let tx_id = TxIdentifier::new(1, 2, Hash256::empty());
        assert_eq!(format!("{tx_id}"), "1.2.a7ffc6f8");
    }

    #[test]
    fn block_id_encode_decode() {
        let block_id = BlockIdentifier::new(1, 2, Hash256::empty());
        let bytes = block_id.to_bytes();
        let result = BlockIdentifier::from_bytes(bytes);
        assert!(result.is_ok(), "decoding failed: {}", result.unwrap_err());
        assert_eq!(result.unwrap(), block_id);
    }

    #[test]
    fn block_id_display() {
        let block_id = BlockIdentifier::new(1, 2, Hash256::empty());
        assert_eq!(format!("{block_id}"), "1.2.a7ffc6f8");
    }

    #[test]
    fn block_id_ordering() {
        // Simple test
        let first = BlockIdentifier::new(10, 20, Hash256::zero());
        let last = BlockIdentifier::new(1, 2, Hash256::empty());
        assert!(first < last);

        // Do some fuzzing
        for i in 0..1_000u64 {
            let h1 = Hash256::digest(&i.to_be_bytes());
            let h2 = Hash256::digest(&i.to_le_bytes());
            let b1 = BlockIdentifier::new(i as u32, i, h1);
            let b2 = BlockIdentifier::new(i as u32 + 1, i + 1, h2);
            assert_eq!(
                h1 < h2,
                b1 < b2,
                "block identifiers must be sorted based on their hash"
            );
        }
    }

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
    fn decentralized_id_prefix() {
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
            // -> so i can match on byte level agains abcd0000 (in this case 0xd0)
            assert_eq!(back_to_uuid.as_bytes()[0] & 0xF0, 0xd0);
        }
    }

    #[test]
    fn external_id_prefix() {
        for _ in 0..1_000_000 {
            let base_id = Uuid::new_v4();
            let asset_id = ExternalId::from(base_id);
            let aid_string = asset_id.to_string();
            assert_eq!(
                aid_string.chars().next(),
                Some('e'),
                "External-IDs must be prefixed with 'e' in string representation"
            );
            let back_to_uuid: Uuid = asset_id.into();
            assert_ne!(base_id, back_to_uuid);
            // Check, if first four bits match 'e'
            // NOTE: Bit-level-hacking here: bits abcdefgh & 11110000 = abcd0000
            // -> so i can match on byte level agains abcd0000 (in this case 0xe0)
            assert_eq!(back_to_uuid.as_bytes()[0] & 0xF0, 0xe0);
        }
    }

    #[test]
    fn differentiate_id_types() {
        // This test ensures, that all ID types generate different bit-level representations.
        // In other words: They do not match, even if I use the same uuid to generate them.
        // This allows us to easily spot, which ID type we have and prevents cross-matches.
        for _ in 0..1_000_000 {
            let base_id = Uuid::new_v4();
            let agent_id: Uuid = AgentId::from(base_id).into();
            let borderless_id: Uuid = BorderlessId::from(base_id).into();
            let contract_id: Uuid = ContractId::from(base_id).into();
            let did: Uuid = Did::from(base_id).into();
            let external_id: Uuid = ExternalId::from(base_id).into();
            let flow_id: Uuid = FlowId::from(base_id).into();
            let ids = [
                base_id,
                agent_id,
                borderless_id,
                contract_id,
                did,
                external_id,
                flow_id,
            ];
            // NOTE: Check all permutations, just to be sure:
            for i in 0..ids.len() {
                for j in i..ids.len() {
                    assert_eq!(ids[i] == ids[j], i == j);
                }
            }
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

    #[test]
    fn external_id_construction() {
        for _ in 0..1_000_000 {
            let base_id = Uuid::new_v4();
            let base_u128 = base_id.as_u128();
            let from_uuid = ExternalId::from(base_id);
            let from_u128 = ExternalId::from(base_u128);
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
    fn check_aid_prefix() {
        for _ in 0..1_000_000 {
            assert!(aid_prefix(&AgentId::generate()));
            assert!(!aid_prefix(&BorderlessId::generate()));
            assert!(!aid_prefix(&ContractId::generate()));
            assert!(!aid_prefix(&Did::generate()));
            assert!(!aid_prefix(&ExternalId::generate()));
            assert!(!aid_prefix(&FlowId::generate()));
            assert!(!aid_prefix(&[]));
        }
    }

    #[test]
    fn check_bid_prefix() {
        for _ in 0..1_000_000 {
            assert!(!bid_prefix(&AgentId::generate()));
            assert!(bid_prefix(&BorderlessId::generate()));
            assert!(!bid_prefix(&ContractId::generate()));
            assert!(!bid_prefix(&Did::generate()));
            assert!(!bid_prefix(&ExternalId::generate()));
            assert!(!bid_prefix(&FlowId::generate()));
            assert!(!bid_prefix(&[]));
        }
    }

    #[test]
    fn check_cid_prefix() {
        for _ in 0..1_000_000 {
            assert!(!cid_prefix(&AgentId::generate()));
            assert!(!cid_prefix(&BorderlessId::generate()));
            assert!(cid_prefix(&ContractId::generate()));
            assert!(!cid_prefix(&Did::generate()));
            assert!(!cid_prefix(&ExternalId::generate()));
            assert!(!cid_prefix(&FlowId::generate()));
            assert!(!cid_prefix(&[]));
        }
    }

    #[test]
    fn check_did_prefix() {
        for _ in 0..1_000_000 {
            assert!(!did_prefix(&AgentId::generate()));
            assert!(!did_prefix(&BorderlessId::generate()));
            assert!(!did_prefix(&ContractId::generate()));
            assert!(did_prefix(&Did::generate()));
            assert!(!did_prefix(&ExternalId::generate()));
            assert!(!did_prefix(&FlowId::generate()));
            assert!(!did_prefix(&[]));
        }
    }

    #[test]
    fn check_eid_prefix() {
        for _ in 0..1_000_000 {
            assert!(!eid_prefix(&AgentId::generate()));
            assert!(!eid_prefix(&BorderlessId::generate()));
            assert!(!eid_prefix(&ContractId::generate()));
            assert!(!eid_prefix(&Did::generate()));
            assert!(eid_prefix(&ExternalId::generate()));
            assert!(!eid_prefix(&FlowId::generate()));
            assert!(!eid_prefix(&[]));
        }
    }

    #[test]
    fn check_fid_prefix() {
        for _ in 0..1_000_000 {
            assert!(!fid_prefix(&AgentId::generate()));
            assert!(!fid_prefix(&BorderlessId::generate()));
            assert!(!fid_prefix(&ContractId::generate()));
            assert!(!fid_prefix(&Did::generate()));
            assert!(!fid_prefix(&ExternalId::generate()));
            assert!(fid_prefix(&FlowId::generate()));
            assert!(!fid_prefix(&[]));
        }
    }

    #[test]
    fn string_representation() {
        fn parse<T>(id: T)
        where
            T: Into<String> + TryFrom<String> + fmt::Debug + Eq + Copy,
        {
            let s1: String = id.into();
            let a1: Result<T, _> = s1.try_into();
            assert!(a1.is_ok());
            let id2 = unsafe { a1.unwrap_unchecked() };
            assert_eq!(id, id2);
        }
        parse(AgentId::generate());
        parse(BorderlessId::generate());
        parse(ContractId::generate());
        parse(Did::generate());
        parse(ExternalId::generate());
        parse(FlowId::generate());
    }

    #[test]
    fn uuid_conversion() {
        let uid = Uuid::new_v4();
        let aid: AgentId = uid.into();
        let bid: BorderlessId = uid.into();
        let cid: ContractId = uid.into();
        let did: Did = uid.into();
        let eid: ExternalId = uid.into();
        let fid: FlowId = uid.into();
        // They must never match the uuid, because of the prefix
        assert_ne!(aid.into_bytes(), uid.into_bytes());
        assert_ne!(bid.into_bytes(), uid.into_bytes());
        assert_ne!(cid.into_bytes(), uid.into_bytes());
        assert_ne!(did.into_bytes(), uid.into_bytes());
        assert_ne!(eid.into_bytes(), uid.into_bytes());
        assert_ne!(fid.into_bytes(), uid.into_bytes());
        assert_ne!(aid.into_uuid(), uid);
        assert_ne!(bid.into_uuid(), uid);
        assert_ne!(cid.into_uuid(), uid);
        assert_ne!(did.into_uuid(), uid);
        assert_ne!(eid.into_uuid(), uid);
        assert_ne!(fid.into_uuid(), uid);

        // But roundtrip will work
        assert_eq!(AgentId::from(aid.into_uuid()), aid);
        assert_eq!(BorderlessId::from(aid.into_uuid()), bid);
        assert_eq!(ContractId::from(aid.into_uuid()), cid);
        assert_eq!(Did::from(aid.into_uuid()), did);
        assert_eq!(ExternalId::from(aid.into_uuid()), eid);
        assert_eq!(FlowId::from(aid.into_uuid()), fid);
    }

    #[test]
    fn merge_is_commutative() {
        for _ in 0..1_000_000 {
            let bid_1 = BorderlessId::generate();
            let bid_2 = BorderlessId::generate();
            assert_eq!(bid_1.merge(&bid_2), bid_2.merge(&bid_1));
        }
    }

    #[test]
    fn merge_compact_is_commutative() {
        for _ in 0..1_000_000 {
            let bid_1 = BorderlessId::generate();
            let bid_2 = BorderlessId::generate();
            assert_eq!(bid_1.merge_compact(&bid_2), bid_2.merge_compact(&bid_1));
        }
    }

    #[test]
    fn serde_conversion_is_ensured() {
        fn check_id<T: DeserializeOwned + From<Uuid> + Into<Uuid> + std::fmt::Debug + Eq>(
            base_id: Uuid,
        ) {
            let json_id = serde_json::to_string(&base_id).unwrap();
            let result: Result<T, _> = serde_json::from_str(&json_id);
            assert!(result.is_ok(), "{}", result.unwrap_err());
            let bid = result.unwrap();
            assert_eq!(bid, T::from(base_id));
            assert_ne!(bid.into(), base_id);
        }
        for _ in 0..100_000 {
            let base_id = Uuid::new_v4();
            check_id::<AgentId>(base_id);
            check_id::<BorderlessId>(base_id);
            check_id::<ContractId>(base_id);
            check_id::<Did>(base_id);
            check_id::<ExternalId>(base_id);
            check_id::<FlowId>(base_id);
        }
    }
}
