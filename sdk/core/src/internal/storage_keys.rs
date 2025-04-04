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
//! # use borderless_sdk::internal::storage_keys::*;
//! # use borderless_sdk::ContractId;
//! # let cid = "cc8ca79c-3bbb-89d2-bb28-29636c170387".parse().unwrap();
//! let base = 42;
//! let sub = 99;
//! let user_key = StorageKey::user_key(&cid, base, sub);
//!
//! assert!(user_key.is_user_key());
//! assert_eq!(user_key.contract_id(), cid);
//! assert_eq!(user_key.sub_key(), sub);
//! ```
//!
//! You can use the provided helper function [`blind_base_key`] to ensure, that any random `u64` will always
//! be a user-key and never accidentaly a system-key:
//!
//! ## Example
//!
//! ```rust
//! # use borderless_sdk::internal::storage_keys::*;
//! # use borderless_sdk::ContractId;
//! # let cid = "cc8ca79c-3bbb-89d2-bb28-29636c170387".parse().unwrap();
//! // This is definetely a system-key
//! let key = BASE_KEY_METADATA;
//! assert!(is_system_key(key));
//!
//! let user_key = blind_base_key(key);
//! assert!(is_user_key(user_key));
//!
//! // You don't need to do this, if you use the StorageKey constructor:
//! let storage_key = StorageKey::user_key(&cid, key, 0);
//! assert!(storage_key.is_user_key());
//! ```

use crate::ContractId;

/// Base-Key used to store metadata about the contract
///
/// The metadata includes things like contract-info, description and more.
/// See `METADATA_SUB_KEY_*` for the meaning of different sub-keys.
pub const BASE_KEY_METADATA: u64 = 0;

/// Base-Key used to store the actions of the contract
///
/// The actions are basically an append-only vector stored at the base-key,
/// where the sub-keys (from `0` to `2^64`) contain the dedicated action values.
pub const BASE_KEY_ACTIONS: u64 = 1;

/// Base-Key used to store the log of a contract
///
/// The log of a contract is a ring-buffer, which is stored in the sub-keys.
pub const BASE_KEY_LOGS: u64 = 2;

/// Reserved Base-Key - indicating the maximum possible system-key
///
/// Everything between `0` and `BASE_KEY_RESERVED` can be used to store special
/// values for the contract.
pub const BASE_KEY_RESERVED: u64 = u64::MAX & !(1 << 63); // max. possible system-key

/// A 32-byte storage key constructed from contract ID, base key, and sub key.
///
/// Use [`StorageKey::user_key`] or [`StorageKey::system_key`] to construct values safely.
pub struct StorageKey([u8; 32]);

impl StorageKey {
    /// Creates a user-space storage key.
    ///
    /// Sets the top bit of `base_key` to indicate the user space.
    ///
    /// # Panics
    /// Panics in debug mode if the result is not in the user space.
    pub fn user_key(cid: &ContractId, base_key: u64, sub_key: u64) -> Self {
        let base_key = blind_base_key(base_key);
        debug_assert!(
            is_user_key(base_key) && !is_system_key(base_key),
            "blinded base_key must be in user-space"
        );
        let key = calc_storage_key(cid, base_key, sub_key);
        Self(key)
    }

    /// Creates a system-space storage key.
    ///
    /// Clears the top bit of `base_key` to reserve it for internal/system use.
    ///
    /// # Panics
    /// Panics in debug mode if the result is not in system space.
    pub fn system_key(cid: &ContractId, base_key: u64, sub_key: u64) -> Self {
        let base_key = base_key & !(1 << 63);
        debug_assert!(
            !is_user_key(base_key) && is_system_key(base_key),
            "system base_key must be in system-space"
        );
        let key = calc_storage_key(cid, base_key, sub_key);
        Self(key)
    }

    /// Returns the raw 32-byte representation of the storage key.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Converts the storage key to a hexadecimal string (for logging/debugging).
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Extracts the contract ID from the key.
    pub fn contract_id(&self) -> ContractId {
        let mut cid = [0u8; 16];
        cid.copy_from_slice(&self.0[0..16]);
        ContractId::from_bytes(cid)
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
}

impl AsRef<[u8]> for StorageKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Calculates a storage key from a contract ID, base key, and sub key.
pub fn calc_storage_key(cid: &ContractId, base_key: u64, sub_key: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[0..16].copy_from_slice(cid.0.as_bytes());
    out[16..24].copy_from_slice(&base_key.to_be_bytes());
    out[24..32].copy_from_slice(&sub_key.to_be_bytes());
    out
}

/// Sets the high bit of the base key to mark it as user-space.
pub fn blind_base_key(base_key: u64) -> u64 {
    base_key | (1 << 63)
}

/// Returns `true` if the base key is marked as user-space.
pub fn is_user_key(base_key: u64) -> bool {
    base_key & (1 << 63) != 0
}

/// Returns `true` if the base key is marked as system/internal-space.
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

        assert_eq!(key.contract_id(), cid);
        assert_eq!(key.base_key(), blind_base_key(base));
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

        assert_eq!(key.contract_id(), cid);
        assert_eq!(key.base_key(), base);
        assert_eq!(key.sub_key(), sub);
        assert!(!key.is_user_key());
        assert!(key.is_system_key());
    }

    #[test]
    fn to_hex_format() {
        let cid = "cc8ca79c-3bbb-89d2-bb28-29636c170387"
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
        let blinded = blind_base_key(base);
        assert_eq!(blinded >> 63, 1);
    }

    #[test]
    fn base_key_checks() {
        // Check, if user-space and system-keys are correctly separated
        for _ in 0..1_000_000 {
            let user_key = blind_base_key(rand::random());
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
        assert!(is_system_key(BASE_KEY_ACTIONS));
        assert!(!is_user_key(BASE_KEY_ACTIONS));
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
}
