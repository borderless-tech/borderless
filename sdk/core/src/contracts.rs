use borderless_hash::Hash256;
use borderless_id_types::{AgentId, BlockIdentifier, TxIdentifier, Uuid};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::{Display, Formatter};
use std::{collections::BTreeMap, str::FromStr};

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

// NOTE: We could re-write the participant logic like this
//
// But that's maybe something for later.
//
// pub struct Participant {
//     pub borderless_id: BorderlessId,
//     pub alias: String,
//     pub roles: Vec<String>,
//     pub sinks: Vec<String>,
// }
// { "borderless-id": "4bec7f8e-5074-49a5-9b94-620fb13f12c0", "alias": null, roles": [ "Flipper" ], "sinks": [ "OTHERFLIPPER" ] },

/*
 * Ok, spitballing here:
 *
 * I think the sinks as they are now, are quite OK.
 * The only thing I would change is, that the sinks that the contract itself defines (with the enum),
 * should work differently in the way that they just output their data as plain json,
 * and the sinks (enum below) are used to subscribe to those outputs using the "alias".
 * We should add a "MethodOrId" to each sink; then we are able to build the CallAction struct for the corresponding
 * contract or agent.
 */

/// High level description and information about the contract itself
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Description {
    pub display_name: String,
    pub summary: String,
    #[serde(default)]
    pub legal: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for SemVer {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const ERR: &str = "Failed to parse version string. Expected: major.minor.patch";
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

// Helper module to be able to parse SemVer from normal strings
pub mod semver_as_string {
    use super::*;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &SemVer, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let version_str = format!("{}.{}.{}", value.major, value.minor, value.patch);
        serializer.serialize_str(&version_str)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SemVer, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(serde::de::Error::custom(
                "Expected version in 'major.minor.patch' format",
            ));
        }
        let major = parts[0].parse().map_err(serde::de::Error::custom)?;
        let minor = parts[1].parse().map_err(serde::de::Error::custom)?;
        let patch = parts[2].parse().map_err(serde::de::Error::custom)?;
        Ok(SemVer {
            major,
            minor,
            patch,
        })
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
    #[serde(with = "crate::contracts::semver_as_string")]
    /// SemVer compatible version string
    pub version: SemVer,

    #[serde(default)]
    /// Time when the contract or process was created (milliseconds since unix epoch)
    pub active_since: u64,

    #[serde(default)]
    /// Transaction context of the contract-introduction transaction
    pub tx_ctx_introduction: Option<TxCtx>,

    /// Time when the contract or process was revoked or archived (milliseconds since unix epoch)
    #[serde(default)]
    pub inactive_since: u64,

    #[serde(default)]
    /// Transaction context of the contract-revocation transaction
    pub tx_ctx_revocation: Option<TxCtx>,

    /// Parent of the contract or process (in case the contract was updated / replaced)
    pub parent: Option<Uuid>,
}

/// Contract-Info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub contract_id: ContractId,
    pub participants: Vec<BorderlessId>,
    pub roles: Vec<Role>,
    pub sinks: Vec<Sink>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Id {
    Contract { contract_id: ContractId },
    Agent { agent_id: AgentId },
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Id::Contract { contract_id } => writeln!(f, "{}", contract_id.to_string())?,
            Id::Agent { agent_id } => writeln!(f, "{}", agent_id.to_string())?,
        }
        Ok(())
    }
}

impl Id {
    pub fn as_cid(&self) -> Option<ContractId> {
        match self {
            Id::Contract { contract_id } => Some(*contract_id),
            Id::Agent { .. } => None,
        }
    }

    pub fn as_aid(&self) -> Option<AgentId> {
        match self {
            Id::Contract { .. } => None,
            Id::Agent { agent_id } => Some(*agent_id),
        }
    }

    pub fn contract(contract_id: ContractId) -> Self {
        Id::Contract { contract_id }
    }

    pub fn agent(agent_id: AgentId) -> Self {
        Id::Agent { agent_id }
    }
}

impl AsRef<[u8; 16]> for Id {
    fn as_ref(&self) -> &[u8; 16] {
        match self {
            Id::Contract { contract_id } => contract_id.as_ref(),
            Id::Agent { agent_id } => agent_id.as_ref(),
        }
    }
}

impl PartialEq<ContractId> for Id {
    fn eq(&self, other: &ContractId) -> bool {
        match self {
            Id::Contract { contract_id } => contract_id == other,
            Id::Agent { .. } => false,
        }
    }
}

impl PartialEq<AgentId> for Id {
    fn eq(&self, other: &AgentId) -> bool {
        match self {
            Id::Agent { agent_id } => agent_id == other,
            Id::Contract { .. } => false,
        }
    }
}

impl From<ContractId> for Id {
    fn from(contract_id: ContractId) -> Self {
        Id::Contract { contract_id }
    }
}

impl From<AgentId> for Id {
    fn from(agent_id: AgentId) -> Self {
        Id::Agent { agent_id }
    }
}

/// Specifies the source for some wasm module
///
/// Can be either "remote", when the code can be fetched from our remote repository,
/// or "local" - in this case the compiled module is just serialized as bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WasmSource {
    Remote { repository: String },
    Local { code: Vec<u8> },
}

// TODO: WIP - just to save some ideas
// (the name should also be different)
// -> maybe this should be part of the contract-package crate ?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmModule {
    /// Name of the application (group) that the contract is part of
    pub application: String,

    /// Name of the module inside the application
    pub app_module: String,

    // NOTE: This ensures compatibility with old versions
    #[serde(default)]
    #[serde(with = "crate::contracts::semver_as_string")]
    /// SemVer compatible version string
    pub version: SemVer,

    /// Location, where the compiled module can be obtained
    pub source: WasmSource,
}

/// Introduction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Introduction {
    /// Contract- or Agent-ID
    #[serde(flatten)]
    pub id: Id,

    /// List of participants
    #[serde(default)]
    pub participants: Vec<BorderlessId>,

    /// Bytes of the initial state.
    ///
    /// This will be parsed by the implementors of the contract.
    pub initial_state: Value,

    /// Mapping between users and roles.
    #[serde(default)]
    pub roles: Vec<Role>,

    /// List of available sinks
    #[serde(default)]
    pub sinks: Vec<Sink>,

    /// High-Level description of the contract
    pub desc: Description,

    /// Contract metadata
    pub meta: Metadata,
}

impl Introduction {
    /// Encode the introduction to json bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&self)
    }

    /// Decode the introduction from json bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Pretty-Print the introduction as json
    pub fn pretty_print(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self)
    }
}

impl FromStr for Introduction {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

/// Contract revocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Revocation {
    /// Contract-ID
    pub contract_id: ContractId,

    /// Reason for the revocation
    pub reason: String,
}

impl Revocation {
    /// Encode the revocation to json bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&self)
    }

    /// Decode the revocation from json bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Pretty-Print the revocation as json
    pub fn pretty_print(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self)
    }
}

impl FromStr for Revocation {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
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
    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        postcard::to_allocvec(&self)
    }

    /// Use postcard to decode the `TxCtx`
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error> {
        postcard::from_bytes(bytes)
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
    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        postcard::to_allocvec(&self)
    }

    /// Use postcard to decode the `BlockCtx`
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error> {
        postcard::from_bytes(bytes)
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

/// Generated symbols of a contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbols {
    /// Fields and addresses (storage-keys) of the contract-state
    pub state: BTreeMap<String, u64>,
    /// Method-names and method-ids of all actions
    pub actions: BTreeMap<String, u32>,
}

impl Symbols {
    // TODO: I liked the hex-encoding more, but it also made it harder to debug based on the generated symbols in the contract.
    //
    // We should either use hex everywhere or the raw number everywhere. For now I will use the numbers here, but I would maybe change
    // the macro later to utilize the hex-encoding.
    pub fn from_symbols(state_syms: &[(&str, u64)], action_syms: &[(&str, u32)]) -> Self {
        // NOTE: We use a BTreeMap instead of a hash-map to get sorted keys.
        let mut state = BTreeMap::new();
        for (name, addr) in state_syms {
            state.insert(name.to_string(), *addr);
        }
        let mut actions = BTreeMap::new();
        for (name, addr) in action_syms {
            actions.insert(name.to_string(), *addr);
        }
        Self { state, actions }
    }

    /// Use json to encode the `Symbols`
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Use json to decode the `Symbols`
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn general_id() {
        let cid = r#"{ "contract_id": "cbcd81bb-b90c-8806-8341-fe95b8ede45a" }"#;
        let aid = r#"{ "agent_id": "abcd81bb-b90c-8806-8341-fe95b8ede45a" }"#;
        let parsed: Result<Id, _> = serde_json::from_str(&cid);
        assert!(parsed.is_ok(), "{}", parsed.unwrap_err());
        match parsed.unwrap() {
            Id::Contract { contract_id } => assert_eq!(
                contract_id.to_string(),
                "cbcd81bb-b90c-8806-8341-fe95b8ede45a"
            ),
            Id::Agent { .. } => panic!("result was not an agent-id"),
        }

        let parsed: Result<Id, _> = serde_json::from_str(&aid);
        assert!(parsed.is_ok(), "{}", parsed.unwrap_err());
        match parsed.unwrap() {
            Id::Agent { agent_id } => {
                assert_eq!(agent_id.to_string(), "abcd81bb-b90c-8806-8341-fe95b8ede45a")
            }
            Id::Contract { .. } => panic!("result was not a contract-id"),
        }
    }

    #[test]
    fn parse_introduction() {
        let json = r#"
{
  "contract_id": "cc8ca79c-3bbb-89d2-bb28-29636c170387",
  "participants": [],
  "initial_state": {
    "switch": true,
    "counter": 0,
    "history": []
  },
  "roles": [],
  "sinks": [],
  "desc": {
    "display_name": "flipper",
    "summary": "a flipper contract for testing the abi",
    "legal": null
  },
  "meta": {
    "application": "flipper",
    "app_module": "test",
    "version": "0.1.0"
  }
}
"#;
        let result: Result<Introduction, _> = serde_json::from_str(&json);
        assert!(result.is_ok(), "{}", result.unwrap_err());
        assert_eq!(
            result.unwrap().id,
            Id::Contract {
                contract_id: "cc8ca79c-3bbb-89d2-bb28-29636c170387".parse().unwrap()
            }
        );
        let json = json.replace(r#""contract_id": "c"#, r#""agent_id": "a"#);
        let result: Result<Introduction, _> = serde_json::from_str(&json);
        assert!(result.is_ok(), "{}", result.unwrap_err());
        assert_eq!(
            result.unwrap().id,
            Id::Agent {
                agent_id: "ac8ca79c-3bbb-89d2-bb28-29636c170387".parse().unwrap()
            }
        );
    }
}
