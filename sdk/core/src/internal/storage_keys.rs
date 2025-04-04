use crate::ContractId;

pub const BASE_KEY_METADATA: u64 = 0;
pub const BASE_KEY_ACTIONS: u64 = 1;
pub const BASE_KEY_LOGS: u64 = 2;
pub const BASE_KEY_RESERVED: u64 = u64::MAX & !(1 << 63); // max. possible system-key

pub struct StorageKey([u8; 32]);

impl StorageKey {
    /// Creates a user-space storage key.
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
    pub fn system_key(cid: &ContractId, base_key: u64, sub_key: u64) -> Self {
        let base_key = base_key & !(1 << 63); // Clear the high bit (bit 63)
        debug_assert!(
            !is_user_key(base_key) && is_system_key(base_key),
            "system base_key must be in system-space"
        );
        let key = calc_storage_key(cid, base_key, sub_key);
        Self(key)
    }

    /// Returns the raw key bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Debug helper to view the key as hex.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Extracts the contract ID from the key.
    pub fn contract_id(&self) -> ContractId {
        let mut cid = [0u8; 16];
        cid.copy_from_slice(&self.0[0..16]);
        cid
    }

    /// Extracts the base key (u64) from the key.
    pub fn base_key(&self) -> u64 {
        u64::from_be_bytes(self.0[16..24].try_into().expect("base_key slice invalid"))
    }

    /// Extracts the sub key (u64) from the key.
    pub fn sub_key(&self) -> u64 {
        u64::from_be_bytes(self.0[24..32].try_into().expect("sub_key slice invalid"))
    }

    /// Returns 'true' if the keyspace of the base-key is meant for users
    pub fn is_user_key(&self) -> bool {
        is_user_key(self.base_key())
    }

    /// Returns 'true' if the keyspace of the base-key is meant for system reserved values
    pub fn is_user_key(&self) -> bool {
        is_system_key(self.base_key())
    }
}

impl AsRef<[u8]> for StorageKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Calculates a storage key for the given contract-id, base-key and sub-key
pub fn calc_storage_key(cid: &ContractId, base_key: u64, sub_key: u64) -> [u8; 32] {
    // Prepare storage key
    let mut out = [0u8; 32];
    // The first 16 bytes are the contract-id
    out[0..16].copy_from_slice(cid);
    // Then the field key (aka base-key)
    out[16..24].copy_from_slice(&base_key.to_be_bytes());
    // Then the sub-field key (aka sub-key)
    out[24..32].copy_from_slice(&sub_key.to_be_bytes());
    out
}

/// "Blinds" the base-key, so that the values are never in the reserved range for internal (system) values.
/// Ensures the top bit is set to mark the key as a user key.
pub fn blind_base_key(base_key: u64) -> u64 {
    base_key | (1 << 63)
}

/// Returns `true` if the key is a user-space key
pub fn is_user_key(base_key: u64) -> bool {
    base_key & (1 << 63) != 0
}

/// Returns `true` if the key is a system/internal key
pub fn is_system_key(base_key: u64) -> bool {
    base_key & (1 << 63) == 0
}
