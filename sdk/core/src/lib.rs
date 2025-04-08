pub mod collections;
pub mod contract;
pub mod log;
pub mod storage;

pub use anyhow::{anyhow as new_error, Context, Error, Result};

pub mod serialize {
    pub use serde_json::from_value;
}

use serde::{Deserialize, Serialize};

/// This module is **not** part of the public API.
/// It exists, because the procedural macros and some internal implementations (like the contract runtime) rely on it.
///
/// You probably don't want to use this directly.
#[doc(hidden)]
#[path = "private.rs"]
pub mod __private;

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
/// This mechanism ensures that you cannot mistake an asset-id for e.g. a contract-id and vice versa. Even if you convert the asset-id
/// back into a uuid and the result into a contract-id, the results are different.
///
/// The implementation of the IDs is compliant with [RFC9562](https://www.ietf.org/rfc/rfc9562.html#name-uuid-version-8),
/// as we utilize standard version 8 uuids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct AgentId(uuid::Uuid);
impl_uuid!(AgentId, 0xaF, 0xa0);

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
impl_uuid!(BorderlessId, 0xbF, 0xb0);

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
impl_uuid!(ContractId, 0xcF, 0xc0);

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
impl_uuid!(Did, 0xdF, 0xd0);

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
