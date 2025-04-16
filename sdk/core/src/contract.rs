use std::{fmt::Display, str::FromStr};

use borderless_hash::Hash256;
use borderless_id_types::{AgentId, BlockIdentifier, TxIdentifier, Uuid};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{BorderlessId, ContractId, RoleId};

/// Contract Environment
pub mod env {
    use borderless_id_types::{BlockIdentifier, TxIdentifier};

    use crate::{
        BorderlessId, ContractId,
        __private::{
            read_field, read_register,
            registers::{REGISTER_BLOCK_CTX, REGISTER_TX_CTX, REGISTER_WRITER},
            storage_keys::*,
        },
    };

    use super::{BlockCtx, Description, Metadata, Role, Sink, TxCtx};

    /// Returns the contract-id of the current contract
    pub fn contract_id() -> ContractId {
        read_field(BASE_KEY_METADATA, META_SUB_KEY_CONTRACT_ID)
            .expect("contract-id not in metadata")
    }

    pub fn participants() -> Vec<BorderlessId> {
        read_field(BASE_KEY_METADATA, META_SUB_KEY_PARTICIPANTS)
            .expect("participants not in metadata")
    }

    pub fn roles() -> Vec<Role> {
        read_field(BASE_KEY_METADATA, META_SUB_KEY_ROLES).expect("roles not in metadata")
    }

    pub fn sinks() -> Vec<Sink> {
        read_field(BASE_KEY_METADATA, META_SUB_KEY_SINKS).expect("sinks not in metadata")
    }

    pub fn desc() -> Description {
        read_field(BASE_KEY_METADATA, META_SUB_KEY_DESC).expect("description not in metadata")
    }

    pub fn meta() -> Metadata {
        read_field(BASE_KEY_METADATA, META_SUB_KEY_META).expect("meta not in metadata")
    }

    pub fn writer() -> BorderlessId {
        let bytes = read_register(REGISTER_WRITER).expect("caller not present");
        BorderlessId::from_bytes(bytes.try_into().expect("caller must be a borderless-id"))
    }

    pub fn tx_ctx() -> TxCtx {
        let bytes = read_register(REGISTER_TX_CTX).expect("tx-id not present");
        TxCtx::from_bytes(&bytes).expect("invalid data-model in tx-id register")
    }

    pub fn tx_id() -> TxIdentifier {
        let bytes = read_register(REGISTER_TX_CTX).expect("tx-id not present");
        TxCtx::from_bytes(&bytes)
            .expect("invalid data-model in tx-id register")
            .tx_id
    }

    pub fn tx_index() -> u64 {
        let bytes = read_register(REGISTER_TX_CTX).expect("tx-id not present");
        TxCtx::from_bytes(&bytes)
            .expect("invalid data-model in tx-id register")
            .index
    }

    pub fn block_ctx() -> BlockCtx {
        let bytes = read_register(REGISTER_BLOCK_CTX).expect("block-id not present");
        BlockCtx::from_bytes(&bytes).expect("invalid data-model in block-ctx register")
    }

    pub fn block_id() -> BlockIdentifier {
        let bytes = read_register(REGISTER_BLOCK_CTX).expect("block-id not present");
        BlockCtx::from_bytes(&bytes)
            .expect("invalid data-model in block-ctx register")
            .block_id
    }

    pub fn block_timestamp() -> u64 {
        let bytes = read_register(REGISTER_BLOCK_CTX).expect("block-timestamp not present");
        BlockCtx::from_bytes(&bytes)
            .expect("invalid data-model in block-ctx register")
            .timestamp
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MethodOrId {
    ByName { method: String },
    ById { method_id: u32 }, // < TODO Use first bit for blinding here, to distinguish user and system actions ?
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Sink {
    Contract {
        contract_id: ContractId,
        alias: String,
        restrict_to_users: Vec<BorderlessId>,
    },
    Process {
        process_id: AgentId,
        alias: String,
        owner: BorderlessId,
    },
}

impl Sink {
    /// Creates a new Sink for a SmartProcess
    pub fn process(process_id: AgentId, alias: String, owner: BorderlessId) -> Sink {
        Sink::Process {
            process_id,
            alias: alias.to_ascii_uppercase(),
            owner,
        }
    }

    /// Creates a new Sink for a SmartContract
    pub fn contract(
        contract_id: ContractId,
        alias: String,
        restrict_to_users: Vec<BorderlessId>,
    ) -> Sink {
        Sink::Contract {
            contract_id,
            alias: alias.to_ascii_uppercase(),
            restrict_to_users,
        }
    }

    /// Consumes the sink and returns the same sink,
    /// but with alias converted to ascii-uppercase.
    pub fn ensure_uppercase_alias(self) -> Self {
        match self {
            Sink::Contract {
                contract_id,
                alias,
                restrict_to_users,
            } => Sink::Contract {
                contract_id,
                alias: alias.to_ascii_uppercase(),
                restrict_to_users,
            },
            Sink::Process {
                process_id,
                alias,
                owner,
            } => Sink::Process {
                process_id,
                alias: alias.to_ascii_uppercase(),
                owner,
            },
        }
    }

    /// Checks weather or not the given user has access to this sink
    pub fn has_access(&self, user: BorderlessId) -> bool {
        match self {
            Sink::Process { owner, .. } => *owner == user,
            Sink::Contract {
                restrict_to_users, ..
            } => {
                // If the vector is empty, everyone has access
                restrict_to_users.is_empty() || restrict_to_users.iter().any(|u| *u == user)
            }
        }
    }

    pub fn alias(&self) -> String {
        match self {
            Sink::Process { alias, .. } => alias.to_ascii_uppercase(),
            Sink::Contract { alias, .. } => alias.to_ascii_uppercase(),
        }
    }

    pub fn is_process(&self) -> bool {
        match self {
            Sink::Process { .. } => true,
            Sink::Contract { .. } => false,
        }
    }
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
    #[serde(with = "crate::contract::semver_as_string")]
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
    pub initial_state: Value,

    /// Mapping between users and roles.
    pub roles: Vec<Role>,

    // TODO: Re-Think Concept of sinks
    /// List of available sinks
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

/// Holds transaction data that shall be send "outwards" to the p2p network.
///
/// The receiver of an `OutTx` can use the contract-id and data fields
/// to generate a transaction for the p2p network.
#[derive(Debug, Clone)]
pub struct OutTx {
    pub contract_id: ContractId,
    pub action: CallAction,
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
}
