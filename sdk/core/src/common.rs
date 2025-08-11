use anyhow::{anyhow, Context};
use borderless_id_types::{AgentId, Uuid};
use borderless_pkg::WasmPkg;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, fmt::Display, str::FromStr};

pub use borderless_pkg as pkg;

use crate::{contracts::TxCtx, events::Sink, events::Topic, BorderlessId, ContractId};

/// High level description and information about the contract or agent itself
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Description {
    pub display_name: String,
    pub summary: String,
    #[serde(default)]
    pub legal: Option<String>,
}

/// Metadata of the contract or process.
///
/// Used for administration purposes.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(default)]
    /// Time when the contract or process was created (milliseconds since unix epoch)
    pub active_since: u64,

    #[serde(default)]
    /// Transaction context of the contract-introduction transaction
    ///
    /// Is `None`, if the entity is not a contract.
    pub tx_ctx_introduction: Option<TxCtx>,

    /// Time when the contract or process was revoked or archived (milliseconds since unix epoch)
    #[serde(default)]
    pub inactive_since: u64,

    #[serde(default)]
    /// Transaction context of the contract-revocation transaction (only for contracts)
    ///
    /// Is `None`, if the entity is not a contract.
    pub tx_ctx_revocation: Option<TxCtx>,

    /// Parent of the contract or process (in case the contract / agent was updated or replaced by a newer version)
    #[serde(default)]
    pub parent: Option<Uuid>,
}

/// Generalized ID-Tag for contracts and agents
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Id {
    Contract { contract_id: ContractId },
    Agent { agent_id: AgentId },
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

    #[cfg(feature = "generate_ids")]
    pub fn generate(pkg_type: &borderless_pkg::PkgType) -> Self {
        match pkg_type {
            borderless_pkg::PkgType::Contract => Id::Contract {
                contract_id: ContractId::generate(),
            },
            borderless_pkg::PkgType::Agent => Id::Agent {
                agent_id: AgentId::generate(),
            },
        }
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Id::Contract { contract_id } => write!(f, "{contract_id}"),
            Id::Agent { agent_id } => write!(f, "{agent_id}"),
        }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub id: BorderlessId,
    pub alias: String,
    /// Roles of the user (only relevant for contracts)
    #[serde(default)]
    pub roles: Vec<String>,
}
// { "borderless-id": "4bec7f8e-5074-49a5-9b94-620fb13f12c0", "alias": null, roles": [ "Flipper" ]},

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

/// An introduction of either a contract or agent
///
/// There are no two distinct types, since the similarities between contracts and agents are quite big.
/// The main difference is, that agents have no roles attached to them and are not introduced or revoked by a transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Introduction {
    /// Contract- or Agent-ID
    #[serde(flatten)]
    pub id: Id,

    /// List of participants
    #[serde(default)]
    pub participants: Vec<Participant>,

    /// Initial state as JSON value
    ///
    /// This will be parsed by the implementors of the contract or agent
    pub initial_state: Value,

    /// List of available sinks (only relevant for contracts)
    #[serde(default)]
    pub sinks: Vec<Sink>,

    /// List of available subscriptions (only relevant for agents)
    #[serde(default)]
    pub subscriptions: Vec<Topic>,

    /// High-Level description of the contract or agent
    pub desc: Description,

    #[serde(default)]
    /// metadata of the contract or agent
    pub meta: Metadata,

    /// Definition of the wasm package for this contract or agent
    pub package: WasmPkg,
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

/// Digital-Tranfer-Object (Dto) of an [`Introduction`]
///
/// When new contracts or agents are created via web-api, things like [`Metadata`] do not make sense yet.
/// This DTO omits the metadata and makes the [`Id`] optional, so a new [`Id`] can be generated for the package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntroductionDto {
    /// Optional Contract- or Agent-ID
    ///
    /// If this field is empty, a new ID will be generated for the contract or agent.
    #[serde(flatten)]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,

    /// List of participants
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub participants: Vec<Participant>,

    /// Initial state as JSON value
    ///
    /// This will be parsed by the implementors of the contract or agent
    pub initial_state: Value,

    /// List of available sinks
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sinks: Vec<SinkDto>,

    /// List of available topics
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subscriptions: Vec<Topic>,

    /// High-Level description of the contract or agent
    pub desc: Description,

    /// Definition of the wasm package for this contract or agent
    pub package: WasmPkg,
}

impl TryFrom<IntroductionDto> for Introduction {
    type Error = crate::Error;

    fn try_from(value: IntroductionDto) -> Result<Self, Self::Error> {
        let id = {
            #[cfg(feature = "generate_ids")]
            {
                Id::generate(&value.package.pkg_type)
            }
            #[cfg(not(feature = "generate_ids"))]
            {
                value.id.with_context(|| {
                    "ID must be set - enable feature 'generate_ids' to autogenerate an ID"
                })?
            }
        };

        let sinks: Vec<Sink> = value.sinks.into_iter().map(|s| s.into()).collect();
        match id {
            Id::Contract { .. } => {
                if sinks.iter().any(|s| s.writer.is_empty()) {
                    return Err(anyhow!(
                        "Sinks defined in a smart contract must contain a writer"
                    ));
                }
                if value.participants.is_empty() {
                    return Err(anyhow!("Smart contracts must contain participants"));
                }
                if !value.subscriptions.is_empty() {
                    return Err(anyhow!("Smart contracts do not support subscriptions"));
                }
            }
            Id::Agent { .. } => {
                if sinks.iter().any(|s| !s.writer.is_empty()) {
                    return Err(anyhow!(
                        "Sinks defined in a sw-agent must NOT contain a writer"
                    ));
                }
                if !value.participants.is_empty() {
                    return Err(anyhow!("Sw-Agents must NOT contain participants"));
                }
            }
        }

        Ok(Self {
            id,
            participants: value.participants,
            initial_state: value.initial_state,
            sinks,
            subscriptions: value.subscriptions,
            desc: value.desc,
            meta: Default::default(),
            package: value.package,
        })
    }
}

impl FromStr for IntroductionDto {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

/// Digital-Tranfer-Object (Dto) of a [`Sink`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkDto {
    /// Contract-ID of the sink
    pub contract_id: ContractId,

    /// Alias for the sink
    ///
    /// Sinks can be accessed by their alias, allowing an easier lookup.
    pub alias: String,

    /// Participant-Alias of the writer
    ///
    /// All transactions for this `Sink` will be written by this writer.
    /// Sinks defined in a sw-agent have no writers, as agents have no participants
    pub writer: Option<String>,
}

impl From<SinkDto> for Sink {
    fn from(value: SinkDto) -> Self {
        Self {
            contract_id: value.contract_id,
            alias: value.alias,
            // Defaults to empty string if no writer is provided
            writer: value.writer.unwrap_or_default(),
        }
    }
}

/// Contract revocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Revocation {
    /// Contract- or Agent-ID
    #[serde(flatten)]
    pub id: Id,

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
  "meta": {},
  "package": {
     "name": "flipper-contract",
     "pkg_type": "contract",
     "source": {
        "version": "0.1.0",
        "digest": "",
        "wasm": ""
     }
  }
}
"#;
        let result: Result<Introduction, _> = serde_json::from_str(&json);
        assert!(result.is_ok(), "{}", result.unwrap_err());
        let introduction = result.unwrap();
        assert_eq!(
            introduction.id,
            Id::Contract {
                contract_id: "cc8ca79c-3bbb-89d2-bb28-29636c170387".parse().unwrap()
            }
        );
        let json = json.replace(r#""contract_id": "c"#, r#""agent_id": "a"#);
        let result: Result<Introduction, _> = serde_json::from_str(&json);
        assert!(result.is_ok(), "{}", result.unwrap_err());
        let introduction = result.unwrap();
        assert_eq!(
            introduction.id,
            Id::Agent {
                agent_id: "ac8ca79c-3bbb-89d2-bb28-29636c170387".parse().unwrap()
            }
        );
    }
}
