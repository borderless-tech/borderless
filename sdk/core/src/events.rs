use anyhow::anyhow;
use borderless_id_types::{AgentId, BorderlessId, ContractId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt::Display, str::FromStr};

use crate::events::private::Sealed;
use crate::prelude::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
/// Enum to represent the type of method-call
pub enum MethodOrId {
    /// Method is called by its name
    ByName { method: String },
    /// Method is called by its id
    ById { method_id: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Data-model for an action-call in contracts and agents.
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
    /// Create a new `CallAction`
    pub fn new(method: MethodOrId, params: Value) -> Self {
        Self { method, params }
    }

    /// Create a new `CallAction` by method-name
    pub fn by_method(method_name: impl AsRef<str>, params: Value) -> Self {
        Self {
            method: MethodOrId::ByName {
                method: method_name.as_ref().to_string(),
            },
            params,
        }
    }

    /// Create a new `CallAction` by method-id
    pub fn by_method_id(method_id: u32, params: Value) -> Self {
        Self {
            method: MethodOrId::ById { method_id },
            params,
        }
    }

    /// Returns the method-name of this action (if any)
    pub fn method_name(&self) -> Option<&str> {
        match &self.method {
            MethodOrId::ByName { method } => Some(method.as_str()),
            MethodOrId::ById { .. } => None,
        }
    }

    /// Returns the method-id of this action (if any)
    pub fn method_id(&self) -> Option<u32> {
        match self.method {
            MethodOrId::ByName { .. } => None,
            MethodOrId::ById { method_id } => Some(method_id),
        }
    }

    /// Prints either the method-name or method-id for this action
    pub fn print_method(&self) -> String {
        match &self.method {
            MethodOrId::ByName { method } => format!("method-name={method}"),
            MethodOrId::ById { method_id } => format!("method-id={method_id}"),
        }
    }

    /// Deserializes the JSON-Bytes into a `CallAction`
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Pretty-prints the entire `CallAction` as JSON
    pub fn pretty_print(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self)
    }

    /// Serialized the `CallAction` into JSON-Bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&self)
    }
}

pub(crate) struct Init;
struct WithAction;

/// Builder to create a new `ContractCall`
pub struct CallBuilder<STATE> {
    pub(crate) id: ContractId,
    pub(crate) name: String,
    pub(crate) writer: Option<BorderlessId>,
    pub(crate) action: Option<CallAction>,
    _marker: std::marker::PhantomData<STATE>,
}

impl CallBuilder<Init> {
    pub fn new(id: ContractId, method_name: &str) -> CallBuilder<Init> {
        CallBuilder {
            id,
            name: method_name.to_string(),
            writer: None,
            action: None,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn with_value(self, value: Value) -> CallBuilder<WithAction> {
        let action = CallAction::by_method(&self.name, value);
        CallBuilder {
            id: self.id,
            name: self.name,
            writer: None,
            action: Some(action),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn with_args<T: serde::Serialize>(
        self,
        args: T,
    ) -> Result<CallBuilder<WithAction>, crate::Error> {
        let value = serde_json::to_value(args).map_err(|e| {
            crate::Error::msg(format!("failed to convert args for method-call: {e}"))
        })?;
        let action = CallAction::by_method(&self.name, value);
        Ok(CallBuilder {
            id: self.id,
            name: self.name,
            writer: None,
            action: Some(action),
            _marker: std::marker::PhantomData,
        })
    }
}

impl CallBuilder<WithAction> {
    pub fn with_writer(
        self,
        writer_alias: impl AsRef<str>,
    ) -> Result<CallBuilder<WithAction>, crate::Error> {
        // Check if a participant with the provided alias exists
        let writer_id = env::participant(writer_alias.as_ref())?;
        Ok(CallBuilder {
            id: self.id,
            name: self.name,
            writer: Some(writer_id),
            action: self.action,
            _marker: std::marker::PhantomData::default(),
        })
    }

    pub fn build(self) -> Result<ContractCall, crate::Error> {
        // Fetch the sinks related to the contract
        let mut sinks: Vec<Sink> = env::sinks()
            .into_iter()
            .filter(|s| s.contract_id == self.id)
            .collect();

        // Retain the sinks with our writer
        if let Some(call_writer) = self.writer {
            sinks.retain(|s: &Sink| {
                let alias = s.writer.clone();
                let sink_writer = env::participant(alias).expect("Writer must exist");
                call_writer == sink_writer
            });
        }

        let writer = match sinks.len() {
            0 => return Err(anyhow!("No sink with specified contract and writer found")),
            1 => {
                let sink = sinks.pop().unwrap();
                env::participant(sink.writer)?
            }
            _ => return Err(anyhow!("The writer has multiple sinks")),
        };

        Ok(ContractCall {
            contract_id: self.id,
            action: self.action.unwrap(),
        })
    }
}

/// An outgoing event for another contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractCall {
    pub contract_id: ContractId,
    pub action: CallAction,
    // pub writer: BorderlessId,
}

/// An outgoing event for another agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCall {
    pub agent_id: AgentId,
    pub action: CallAction,
}

/// An outgoing message that clients or agents can subscribe to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub topic: String,
    pub value: Value,
}

/// Convenience function to generate messages
///
/// Returns a [`MsgBuilder`], which can be used to generate messages:
/// ```
/// # use borderless::prelude::*;
/// // Build a new message for topic "/base/nested"
/// let msg = message("/base/nested")
///    .with_value(value!({ "switch": false }));
/// ```
///
/// Note: All topics are prepended with `/{contract-id}` (or `/{agent-id}`),
/// so your topic `my-topic` would become e.g. `/cc963345-4cd9-8f30-b215-2cdffee3d189/my-topic`.
/// This way we distinguish identical topic names for different contracts or agents.
/// Trailing slashes are ignored, so `/my-topic` and `my-topic` would result in an identical topic string.
///
/// Also be aware, that topic subscriptions and matchings are case-insensitive.
///
/// All of these topics would be identical:
/// - `/my-topic`
/// - `my-topic`
/// - `/My-Topic`
/// - `MY-TOPIC`
pub fn message(topic: impl AsRef<str>) -> MsgBuilder {
    MsgBuilder {
        topic: topic.as_ref().to_string(),
    }
}

pub struct MsgBuilder {
    topic: String,
}

impl MsgBuilder {
    pub fn with_value(self, value: serde_json::Value) -> Message {
        Message {
            topic: self.topic,
            value,
        }
    }

    pub fn with_serde<T: serde::Serialize>(self, args: T) -> Result<Message, crate::Error> {
        let value = serde_json::to_value(args).map_err(|e| {
            crate::Error::msg(format!(
                "failed to serialize argument for message on topic '{}': {e}",
                self.topic,
            ))
        })?;
        Ok(Message {
            topic: self.topic,
            value,
        })
    }
}

/// Output Events generated by a contract or sw-agent
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Events {
    pub contracts: Vec<ContractCall>,
    pub local: Vec<AgentCall>,
    pub msgs: Vec<Message>,
}

impl Events {
    /// Returns `true` if there are no events at all
    pub fn is_empty(&self) -> bool {
        self.contracts.is_empty() && self.local.is_empty()
    }

    /// Decodes the `Events` with [`serde_json`]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Encodes the `Events` with [`serde_json`]
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

impl From<ContractCall> for Events {
    fn from(value: ContractCall) -> Self {
        Events {
            contracts: vec![value],
            local: Vec::new(),
            msgs: Vec::new(),
        }
    }
}

impl From<AgentCall> for Events {
    fn from(value: AgentCall) -> Self {
        Events {
            contracts: Vec::new(),
            local: vec![value],
            msgs: Vec::new(),
        }
    }
}

impl From<Message> for Events {
    fn from(value: Message) -> Self {
        Events {
            contracts: Vec::new(),
            local: Vec::new(),
            msgs: vec![value],
        }
    }
}

impl From<Vec<ContractCall>> for Events {
    fn from(value: Vec<ContractCall>) -> Self {
        Events {
            contracts: value,
            local: Vec::new(),
            msgs: Vec::new(),
        }
    }
}

impl From<Vec<AgentCall>> for Events {
    fn from(value: Vec<AgentCall>) -> Self {
        Events {
            contracts: Vec::new(),
            local: value,
            msgs: Vec::new(),
        }
    }
}

impl From<Vec<Message>> for Events {
    fn from(value: Vec<Message>) -> Self {
        Events {
            contracts: Vec::new(),
            local: Vec::new(),
            msgs: value,
        }
    }
}

/// Specifies the Sink-Type of an `ActionOutput`.
///
/// A sink can be either a named sink, that gets referenced by its `sink_alias`.
/// The real contract- or process-id is taken from the Contract- or ProcessInfo,
/// using [`ContractInfo::find_sink`] (or [`ProcessInfo::find_sink`]).
///
/// In general it is recommended to use the named sink-type, as it provides the most
/// comfort and fool-proof way of interacting with other contracts or processes.
///
/// However, for maximum flexibility, users can also refer to a sink directly by their
/// [`ContractId`] or [`ProcessId`].
#[derive(Debug)]
pub enum SinkType {
    Named(String),
    Agent(AgentId),
    Contract(ContractId),
}

impl Display for SinkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SinkType::Named(s) => write!(f, "{s}"),
            SinkType::Agent(s) => write!(f, "{s}"),
            SinkType::Contract(s) => write!(f, "{s}"),
        }
    }
}

/// Trait that indicates that a return type can be used as an output of an action function.
///
/// Note: This trait converts `()`, `ActionOutput`, `Result<(), E>` and `Result<ActionOutput, E>` into [`Events`].
/// The implementation of `ActionOutput` also checks, if the writer actually has access to a sink.
pub trait ActionOutput: private::Sealed {
    fn convert_out_events(self) -> crate::Result<Events>;
}

mod private {
    pub trait Sealed {}
}

impl private::Sealed for () {}
impl ActionOutput for () {
    fn convert_out_events(self) -> crate::Result<Events> {
        Ok(Events::default())
    }
}

impl<E> private::Sealed for Result<(), E> where E: std::fmt::Display + Send + Sync + 'static {}
impl<E> ActionOutput for Result<(), E>
where
    E: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
{
    fn convert_out_events(self) -> crate::Result<Events> {
        self.map_err(|e| crate::Error::msg(e))?.convert_out_events()
    }
}

// TODO We have to implement this on a bunch of different types:
// Events
// ContractCall
// Vec<ContractCall>
//
// .. and their crate::Result<T> equivalents

impl Sealed for ContractCall {}
impl ActionOutput for ContractCall {
    fn convert_out_events(self) -> crate::Result<Events> {
        Ok(Events::from(self))
    }
}

/*
impl<E> private::Sealed for Result<ActionOutput, E> where
    E: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static
{
}
impl<E> ActionOutEvent for Result<ActionOutput, E>
where
    E: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
{
    fn convert_out_events(self) -> crate::Result<Events> {
        let inner = self.map_err(|e| crate::Error::msg(e))?;
        inner.convert_out_events()
    }
}
 */

/// An event Sink for a smart-contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sink {
    /// Contract-ID of the sink
    pub contract_id: ContractId,
    /// Alias for the sink
    ///
    /// Sinks can be accessed by their alias, allowing an easier lookup.
    pub alias: String,
    /// Participant-Alias of the writer
    ///
    /// All transactions for this `Sink` will be written by this writer.
    pub writer: String,
}

impl Sink {
    /// Creates a new Sink for a SmartContract
    pub fn new(contract_id: ContractId, alias: String, writer: String) -> Sink {
        Sink {
            contract_id,
            alias,
            writer,
        }
    }

    /// Checks the alias of the sink against some string
    ///
    /// Note: The casing is ignored here, as it should be in all alias lookups.
    pub fn has_alias(&self, alias: impl AsRef<str>) -> bool {
        alias.as_ref().eq_ignore_ascii_case(&self.alias)
    }
}
