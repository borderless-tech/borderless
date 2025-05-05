//! # StorageKey Module
//!
//! This module defines a structured, namespaced key format used for key-value storage in
//! smart contracts or WASM-based environments.
//!
//! Each key is exactly 32 bytes (`[u8; 32]`) and consists of:
//!
//! ```text
//! [   contract-id (16 bytes)   |   base-key (8 bytes)   |   sub-key (8 bytes)   ]
//! ```
//!
//! - The **contract ID** uniquely identifies a contract's storage space.
//! - The **base-key** defines the primary entry (field).
//! - The **sub-key** allows sub-categorization within the base key.
//!
//! The **highest bit** of the `base-key` is used to distinguish between **user** and **system** keyspace:
//!
//! - If the top bit is `1`, the key is in the user space.
//! - If the top bit is `0`, the key is reserved for internal/system use.
//!
//! This split ensures that user code cannot accidentally collide with internal storage keys.
//!
//! ## Example
//!
//! ```rust
//! # use borderless::__private::storage_keys::*;
//! # use borderless::ContractId;
//! # let cid: ContractId = "cc8ca79c-3bbb-89d2-bb28-29636c170387".parse().unwrap();
//! let base = 42;
//! let sub = 99;
//! let user_key = StorageKey::user_key(&cid, base, sub);
//!
//! assert!(user_key.is_user_key());
//! assert_eq!(user_key.contract_id().unwrap(), cid);
//! assert_eq!(user_key.sub_key(), sub);
//! ```
//!
//! You can use the provided helper function [`make_user_key`] to ensure, that any random `u64` will always
//! be a user-key and never accidentaly a system-key:
//!
//! ## Example
//!
//! ```rust
//! # use borderless::__private::storage_keys::*;
//! # use borderless::ContractId;
//! # let cid: ContractId = "cc8ca79c-3bbb-89d2-bb28-29636c170387".parse().unwrap();
//! // This is definetely a system-key
//! let key = BASE_KEY_METADATA;
//! assert!(is_system_key(key));
//!
//! let user_key = make_user_key(key);
//! assert!(is_user_key(user_key));
//!
//! // You don't need to do this, if you use the StorageKey constructor:
//! let storage_key = StorageKey::user_key(&cid, key, 0);
//! assert!(storage_key.is_user_key());
//! ```

use borderless_id_types::{aid_prefix, cid_prefix, AgentId};

use crate::ContractId;

// --- Base-Keys

/// Base-Key used to store metadata about the contract
///
/// The metadata includes things like contract-info, description and more.
/// See `METADATA_SUB_KEY_*` for the meaning of different sub-keys.
pub const BASE_KEY_METADATA: u64 = 0;

/// Base-Key used to store the actions of the contract
///
/// The actions are basically an append-only vector stored at the base-key,
/// where the sub-keys (from `0` to `2^64`) contain the dedicated action values.
pub const BASE_KEY_ACTION_LOG: u64 = 1;

/// Base-Key used to store the log of a contract
///
/// The log of a contract is a ring-buffer, which is stored in the sub-keys.
pub const BASE_KEY_LOGS: u64 = 2;

/// Base-Key used to performance-metrics of a contract
///
/// These work similar to the logs by using a ring-buffer that is stored in sub-keys.
pub const BASE_KEY_METRICS: u64 = 3;

// TODO: Instead of embedding this in the contract keyspace,
// we could use a global ledger between parties for all contracts.
/// Reserved key-space for ledgers
///
/// Everything from in the range from 2^16 to 2^63 is reserved to store the ledgers of a contract.
pub const BASE_KEY_MASK_LEDGER: u64 = 0x0FFFFFFFFFFF0000;

/// Reserved Base-Key - indicating the maximum possible system-key
///
/// Everything between `0` and `BASE_KEY_RESERVED` can be used to store special
/// values for the contract.
pub const BASE_KEY_RESERVED: u64 = u64::MAX & !(1 << 63); // max. possible system-key

// --- NOTE: The META_SUB_*-keys are basically the values of the introduction
/// Sub-Key to store the contract-id
pub const META_SUB_KEY_CONTRACT_ID: u64 = 0;

/// Sub-Key to store the list of participants
///
/// Expected data-model: `Vec<BorderlessId>`
pub const META_SUB_KEY_PARTICIPANTS: u64 = 1;

/// Sub-Key to store the list of roles
///
/// Expected data-model: `Vec<Role>`
pub const META_SUB_KEY_ROLES: u64 = 2;

/// Sub-Key to store the list of available sinks
///
/// Expected data-model: `Vec<Sink>`
pub const META_SUB_KEY_SINKS: u64 = 3;

/// Sub-Key to store the contract description
///
/// Expected data-model: `Description`
pub const META_SUB_KEY_DESC: u64 = 4;

/// Sub-Key to store the contract metadata
///
/// Expected data-model: `Metadata`
pub const META_SUB_KEY_META: u64 = 5;

/// Sub-Key to store the initial state of the contract
///
/// Expected data-model: `serde_json::Value`
pub const META_SUB_KEY_INIT_STATE: u64 = 6;

/// Sub-Key to store the timestamp, when the contract was revoked.
///
/// Expected data-model: `u64`
///
/// Useful to simply query, if the contract is revoked or not.
/// The timestamp is identical with the timestamp in the `Metadata` field.
pub const META_SUB_KEY_REVOKED_TS: u64 = 7;

/// Sub-Key to store the revocation of the contract.
///
/// Expected data-model: `Revocation`
pub const META_SUB_KEY_REVOCATION: u64 = 8;

/// Reserved Sub-Key - max. possible value.
pub const META_SUB_KEY_RESERVED: u64 = u64::MAX & !(1 << 63);

/// A 32-byte storage key constructed from contract ID, base key, and sub key.
///
/// Use [`StorageKey::user_key`] or [`StorageKey::system_key`] to construct values safely.
pub struct StorageKey([u8; 32]);

impl StorageKey {
    /// Creates a new storage key
    ///
    /// Does not set the high-bits of the key to either the user  or system key-space.
    pub fn new(id: impl AsRef<[u8; 16]>, base_key: u64, sub_key: u64) -> Self {
        let key = calc_storage_key(id.as_ref(), base_key, sub_key);
        Self(key)
    }

    /// Creates a user-space storage key.
    ///
    /// Sets the top bit of `base_key` to indicate the user space.
    ///
    /// # Panics
    /// Panics in debug mode if the result is not in the user space.
    pub fn user_key(id: impl AsRef<[u8; 16]>, base_key: u64, sub_key: u64) -> Self {
        let base_key = make_user_key(base_key);
        debug_assert!(
            is_user_key(base_key) && !is_system_key(base_key),
            "blinded base_key must be in user-space"
        );
        let key = calc_storage_key(id.as_ref(), base_key, sub_key);
        Self(key)
    }

    /// Creates a system-space storage key.
    ///
    /// Clears the top bit of `base_key` to reserve it for internal/system use.
    ///
    /// # Panics
    /// Panics in debug mode if the result is not in system space.
    pub fn system_key(id: impl AsRef<[u8; 16]>, base_key: u64, sub_key: u64) -> Self {
        let base_key = base_key & !(1 << 63);
        debug_assert!(
            !is_user_key(base_key) && is_system_key(base_key),
            "system base_key must be in system-space"
        );
        let key = calc_storage_key(id.as_ref(), base_key, sub_key);
        Self(key)
    }

    /// Returns the raw 32-byte representation of the storage key.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Converts the storage key to a hexadecimal string (for logging/debugging).
    pub fn to_hex(&self) -> String {
        use std::fmt::Write;
        self.0.iter().fold(String::new(), |mut output, b| {
            let _ = write!(output, "{b:02x}");
            output
        })
    }

    /// Extracts the Contract-ID from the key
    ///
    /// Returns `None`, if the key does not belong to a contract.
    pub fn contract_id(&self) -> Option<ContractId> {
        if self.is_contract_key() {
            let id = self.get_id_prefix();
            Some(ContractId::from_bytes(id))
        } else {
            None
        }
    }

    /// Extracts the Agent-ID from the key
    ///
    /// Returns `None`, if the key does not belong to a contract.
    pub fn agent_id(&self) -> Option<AgentId> {
        if self.is_agent_key() {
            let id = self.get_id_prefix();
            Some(AgentId::from_bytes(id))
        } else {
            None
        }
    }

    /// Extracts the ID prefix from the key
    pub fn get_id_prefix(&self) -> [u8; 16] {
        let mut id = [0u8; 16];
        id.copy_from_slice(&self.0[0..16]);
        id
    }

    /// Extracts the base key (u64) from the key.
    pub fn base_key(&self) -> u64 {
        u64::from_be_bytes(self.0[16..24].try_into().expect("base_key slice invalid"))
    }

    /// Extracts the sub key (u64) from the key.
    pub fn sub_key(&self) -> u64 {
        u64::from_be_bytes(self.0[24..32].try_into().expect("sub_key slice invalid"))
    }

    /// Returns `true` if the key belongs to the user-space key range.
    pub fn is_user_key(&self) -> bool {
        is_user_key(self.base_key())
    }

    /// Returns `true` if the key belongs to the system/internal reserved space.
    pub fn is_system_key(&self) -> bool {
        is_system_key(self.base_key())
    }

    /// Returns `true` if the key belongs to a contract
    pub fn is_contract_key(&self) -> bool {
        cid_prefix(self.0)
    }

    /// Returns `true` if the key belongs to a sw-agent
    pub fn is_agent_key(&self) -> bool {
        aid_prefix(self.0)
    }
}

impl AsRef<[u8]> for StorageKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Calculates a storage key from a contract ID, base key, and sub key.
pub fn calc_storage_key(id: &[u8; 16], base_key: u64, sub_key: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[0..16].copy_from_slice(id);
    out[16..24].copy_from_slice(&base_key.to_be_bytes());
    out[24..32].copy_from_slice(&sub_key.to_be_bytes());
    out
}

/// Sets the high bit of the base key to mark it as user-space.
#[inline]
pub fn make_user_key(base_key: u64) -> u64 {
    base_key | (1 << 63)
}

/// Returns `true` if the base key is marked as user-space.
#[inline]
pub fn is_user_key(base_key: u64) -> bool {
    base_key & (1 << 63) != 0
}

/// Returns `true` if the base key is marked as system/internal-space.
#[inline]
pub fn is_system_key(base_key: u64) -> bool {
    base_key & (1 << 63) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_key_encoding_and_extraction() {
        let cid = ContractId::generate();
        let base = 42;
        let sub = 99;
        let key = StorageKey::user_key(&cid, base, sub);

        assert!(key.contract_id().is_some());
        assert_eq!(key.contract_id().unwrap(), cid);
        assert_eq!(key.base_key(), make_user_key(base));
        assert_eq!(key.sub_key(), sub);
        assert!(key.is_user_key());
        assert!(!key.is_system_key());
    }

    #[test]
    fn system_key_encoding_and_extraction() {
        let cid = ContractId::generate();
        let base = BASE_KEY_METADATA;
        let sub = 0;
        let key = StorageKey::system_key(&cid, base, sub);

        assert!(key.contract_id().is_some());
        assert_eq!(key.contract_id().unwrap(), cid);
        assert_eq!(key.base_key(), base);
        assert_eq!(key.sub_key(), sub);
        assert!(!key.is_user_key());
        assert!(key.is_system_key());
    }

    #[test]
    fn disjunct_id_prefix() {
        let aid = AgentId::generate();
        let cid = ContractId::generate();
        let base = 42;
        let sub = 99;
        let aid_key = StorageKey::user_key(&aid, base, sub);
        let cid_key = StorageKey::user_key(&cid, base, sub);
        // Keys must not be equal
        assert_ne!(aid_key.0, cid_key.0);
        // Check that storage keys use the correct prefix
        assert!(aid_prefix(&aid_key.0));
        assert!(!aid_prefix(&cid_key.0));
        assert!(!cid_prefix(&aid_key.0));
        assert!(cid_prefix(&cid_key.0));
    }

    #[test]
    fn return_correct_id_type() {
        let aid = AgentId::generate();
        let cid = ContractId::generate();
        let base = 42;
        let sub = 99;
        let aid_key = StorageKey::user_key(&aid, base, sub);
        let cid_key = StorageKey::user_key(&cid, base, sub);
        // Check, that the correct ID-type is returned
        assert!(aid_key.contract_id().is_none());
        assert!(aid_key.agent_id().is_some());
        assert_eq!(aid_key.agent_id().unwrap(), aid);
        assert!(cid_key.contract_id().is_some());
        assert!(cid_key.agent_id().is_none());
        assert_eq!(cid_key.contract_id().unwrap(), cid);
    }

    #[test]
    fn to_hex_format() {
        let cid: ContractId = "cc8ca79c-3bbb-89d2-bb28-29636c170387"
            .parse()
            .expect("valid contract-id");
        let base = 123;
        let sub = 456;
        let key = StorageKey::user_key(&cid, base, sub);

        let hex = key.to_hex();
        assert_eq!(hex.len(), 64);
        // NOTE: The prefix is exactly the contract-id
        assert_eq!(
            hex,
            "cc8ca79c3bbb89d2bb2829636c170387800000000000007b00000000000001c8"
        );
    }

    #[test]
    fn blinding_sets_high_bit() {
        let base: u64 = 12345;
        let blinded = make_user_key(base);
        assert_eq!(blinded >> 63, 1);
    }

    #[test]
    fn base_key_checks() {
        // Check, if user-space and system-keys are correctly separated
        for _ in 0..1_000_000 {
            let user_key = make_user_key(rand::random());
            let system_key = rand::random::<u64>() & !(1 << 63); // Clear the high-bit

            assert!(is_user_key(user_key));
            assert!(!is_user_key(system_key));
            assert!(is_system_key(system_key));
            assert!(!is_system_key(user_key));
        }
    }

    #[test]
    fn is_system_key_metadata() {
        assert!(is_system_key(BASE_KEY_METADATA));
        assert!(!is_user_key(BASE_KEY_METADATA));
    }

    #[test]
    fn is_system_key_actions() {
        assert!(is_system_key(BASE_KEY_ACTION_LOG));
        assert!(!is_user_key(BASE_KEY_ACTION_LOG));
    }

    #[test]
    fn is_system_key_logs() {
        assert!(is_system_key(BASE_KEY_LOGS));
        assert!(!is_user_key(BASE_KEY_LOGS));
    }

    #[test]
    fn is_system_key_reserved() {
        assert!(is_system_key(BASE_KEY_RESERVED));
        assert!(!is_user_key(BASE_KEY_RESERVED));
    }

    #[test]
    fn base_keys_differ() {
        let mut keys = vec![
            BASE_KEY_METADATA,
            BASE_KEY_ACTION_LOG,
            BASE_KEY_LOGS,
            BASE_KEY_METRICS,
            BASE_KEY_RESERVED,
        ];
        let n_keys = keys.len();
        // If two keys would have the same value,
        // the vector lengths would differ after deduplication
        keys.sort();
        keys.dedup(); // requires sorting
        assert_eq!(n_keys, keys.len());
    }
}
