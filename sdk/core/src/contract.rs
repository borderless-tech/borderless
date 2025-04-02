use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{BorderlessId, ContractId, RoleId};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MethodOrId {
    ByName { method: String },
    ById { method_id: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallAction {
    #[serde(flatten)]
    pub method: MethodOrId,
    pub params: Value,
}

impl FromStr for CallAction {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl CallAction {
    pub fn by_method(method_name: impl AsRef<str>, params: Value) -> Self {
        Self {
            method: MethodOrId::ByName {
                method: method_name.as_ref().to_string(),
            },
            params,
        }
    }

    pub fn by_method_id(method_id: u32, params: Value) -> Self {
        Self {
            method: MethodOrId::ById { method_id },
            params,
        }
    }

    pub fn method_name(&self) -> Option<&str> {
        match &self.method {
            MethodOrId::ByName { method } => Some(method.as_str()),
            MethodOrId::ById { .. } => None,
        }
    }

    pub fn method_id(&self) -> Option<u32> {
        match self.method {
            MethodOrId::ByName { .. } => None,
            MethodOrId::ById { method_id } => Some(method_id),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    pub fn pretty_print(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&self)
    }
}

/// Connects a Borderless-ID and Role-ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    pub participant_id: BorderlessId,
    pub role_id: RoleId,
}

/// High level description and information about the contract itself
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Description {
    pub display_name: String,
    pub summary: String,
    pub legal: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ToString for SemVer {
    fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for SemVer {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const ERR: &'static str = "Failed to parse version string. Expected: major.minor.patch";
        let mut pieces = s.split('.');
        let major = u32::from_str(pieces.next().ok_or(ERR)?).map_err(|_| ERR)?;
        let minor = u32::from_str(pieces.next().ok_or(ERR)?).map_err(|_| ERR)?;
        let patch = u32::from_str(pieces.next().ok_or(ERR)?).map_err(|_| ERR)?;
        Ok(SemVer {
            major,
            minor,
            patch,
        })
    }
}

impl Default for SemVer {
    /// Initializes version with "0.1.0"
    fn default() -> Self {
        Self {
            major: 0,
            minor: 1,
            patch: 0,
        }
    }
}

/// Metadata of the contract or process.
///
/// Used for administration purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// Name of the application (group) that the contract is part of
    pub application: String,

    /// Name of the module inside the application
    pub app_module: String,

    // NOTE: This ensures compatibility with old versions
    #[serde(default)]
    /// SemVer compatible version string
    pub version: SemVer,

    #[serde(default)]
    /// Time when the contract or process was created (seconds since unix epoch)
    pub active_since: u64,

    /// Time when the contract or process was revoked or archived (seconds since unix epoch)
    pub inactive_since: Option<u32>,

    /// Parent of the contract or process (in case the contract was updated / replaced)
    pub parent: Option<uuid::Uuid>,
}

/// Contract-Introduction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Introduction {
    /// Contract-ID
    pub contract_id: ContractId,

    /// List of participants
    pub participants: Vec<BorderlessId>,

    /// Bytes of the initial state.
    ///
    /// This will be parsed by the implementors of the contract.
    pub initial_state: Vec<u8>, // < TODO: serde_json::Value or Vec<u8> ?

    /// Mapping between users and roles.
    pub roles: Vec<Role>,

    // TODO: Re-Think Concept of sinks
    // /// List of available sinks
    // pub sinks: Vec<Sink>,
    //
    /// High-Level description of the contract
    pub desc: Description,

    /// Contract metadata
    pub meta: Metadata,
}

impl Introduction {
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&self)
    }
}

impl FromStr for Introduction {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentId, Did};
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

    #[test]
    fn semver_parsing() {
        let version = "1.0.0".parse::<SemVer>();
        assert!(version.is_ok());
        assert_eq!(
            version.unwrap(),
            SemVer {
                major: 1,
                minor: 0,
                patch: 0
            }
        );
        let version = "asdf".parse::<SemVer>();
        assert!(version.is_err());
        let version = "v1.0.3".parse::<SemVer>();
        assert!(version.is_err());
        let version = "1.0".parse::<SemVer>();
        assert!(version.is_err());
        let version = "1".parse::<SemVer>();
        assert!(version.is_err());
        let version = "1.0.-10".parse::<SemVer>();
        assert!(version.is_err());
    }

    #[test]
    fn semver_to_string() {
        let version = SemVer {
            major: 1,
            minor: 4,
            patch: 113,
        };
        assert_eq!(version.to_string(), "1.4.113".to_string());
        let v2 = "1.4.113".parse().unwrap();
        assert_eq!(version, v2);
    }

    #[test]
    fn semver_default() {
        let v1 = SemVer::default();
        assert_eq!(
            v1,
            SemVer {
                major: 0,
                minor: 1,
                patch: 0
            }
        );
        assert_eq!(v1, "0.1.0".parse().unwrap());
    }
}
