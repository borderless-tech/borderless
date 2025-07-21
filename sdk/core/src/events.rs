use anyhow::anyhow;
use borderless_id_types::{AgentId, BorderlessId, ContractId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt::Display, str::FromStr};

use crate::{common::Id, debug, error, NamedSink};

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

pub struct CallBuilder<ID> {
    pub(crate) id: ID,
    pub(crate) name: String,
}

impl CallBuilder<ContractId> {
    pub fn with_value(self, value: serde_json::Value) -> ContractCall {
        let action = CallAction::by_method(self.name, value);
        ContractCall {
            contract_id: self.id,
            action,
        }
    }

    pub fn with_args<T: serde::Serialize>(self, args: T) -> Result<ContractCall, crate::Error> {
        let value = serde_json::to_value(args).map_err(|e| {
            crate::Error::msg(format!("failed to convert args for method-call: {e}"))
        })?;
        let action = CallAction::by_method(self.name, value);
        Ok(ContractCall {
            contract_id: self.id,
            action,
        })
    }
}

impl CallBuilder<AgentId> {
    pub fn with_value(self, value: serde_json::Value) -> AgentCall {
        let action = CallAction::by_method(self.name, value);
        AgentCall {
            agent_id: self.id,
            action,
        }
    }

    pub fn with_args<T: serde::Serialize>(self, args: T) -> Result<AgentCall, crate::Error> {
        let value = serde_json::to_value(args).map_err(|e| {
            crate::Error::msg(format!("failed to convert args for method-call: {e}"))
        })?;
        let action = CallAction::by_method(self.name, value);
        Ok(AgentCall {
            agent_id: self.id,
            action,
        })
    }
}

// /// Represents a target that should execute some action.
// ///
// /// Since contracts and software-agents both use the [`CallAction`] struct,
// /// but also use different ID types, this enum can be used in cases where a `CallAction`
// /// is bundled with either a [`ContractId`] or [`AgentId`].
// pub enum TargetId {
//     Agent(AgentId),
//     Contract(ContractId),
// }

/// An outgoing event for another contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractCall {
    pub contract_id: ContractId,
    pub action: CallAction,
}

/// An outgoing event for another agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCall {
    pub agent_id: AgentId,
    pub action: CallAction,
}

/// Output Events generated by a contract or sw-agent
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Events {
    pub contracts: Vec<ContractCall>,
    pub local: Vec<AgentCall>,
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
        }
    }
}

impl From<AgentCall> for Events {
    fn from(value: AgentCall) -> Self {
        Events {
            contracts: Vec::new(),
            local: vec![value],
        }
    }
}

impl From<Vec<ContractCall>> for Events {
    fn from(value: Vec<ContractCall>) -> Self {
        Events {
            contracts: value,
            local: Vec::new(),
        }
    }
}

impl From<Vec<AgentCall>> for Events {
    fn from(value: Vec<AgentCall>) -> Self {
        Events {
            contracts: Vec::new(),
            local: value,
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

/// Output events of a contract's action
#[derive(Default)]
#[deprecated]
pub struct ActionOutput {
    actions: Vec<(SinkType, CallAction)>,
}

impl ActionOutput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_event<T: NamedSink>(&mut self, target: T) {
        let (sink_name, action) = target.into_action();
        self.actions
            .push((SinkType::Named(sink_name.to_string()), action));
    }

    /// Adds a generic event to the output - with dynamic dispatch of the output sinks.
    ///
    /// In contrast to [`ActionOutput::add_event`] the event type must only implement `TryInto<CallAction>`,
    /// since the user directly tells us towards which sink the event should be send.
    /// This is only necessary, if the `Sink` has been added after the contract was instantiated.
    pub fn add_event_dynamic<S, IntoAction>(&mut self, sink_alias: S, action: IntoAction)
    where
        S: AsRef<str>,
        IntoAction: TryInto<CallAction>,
        <IntoAction as TryInto<CallAction>>::Error: std::fmt::Display,
    {
        let alias = sink_alias.as_ref().to_string();
        let action = match action.try_into() {
            Ok(a) => a,
            Err(e) => {
                error!("critical error while converting action for dynamic sink '{alias}': {e}");
                crate::__private::abort();
            }
        };
        self.actions.push((SinkType::Named(alias), action))
    }

    pub fn add_event_for_contract<IntoAction>(
        &mut self,
        contract_id: ContractId,
        action: IntoAction,
    ) where
        IntoAction: TryInto<CallAction>,
        <IntoAction as TryInto<CallAction>>::Error: std::fmt::Display,
    {
        let action = match action.try_into() {
            Ok(a) => a,
            Err(e) => {
                error!(
                    "critical error while converting action for dynamic sink '{contract_id}': {e}"
                );
                crate::__private::abort();
            }
        };
        self.actions.push((SinkType::Contract(contract_id), action))
    }

    pub fn add_event_for_process<IntoAction>(&mut self, agent_id: AgentId, action: IntoAction)
    where
        IntoAction: TryInto<CallAction>,
        <IntoAction as TryInto<CallAction>>::Error: std::fmt::Display,
    {
        let action = match action.try_into() {
            Ok(a) => a,
            Err(e) => {
                error!("critical error while converting action for dynamic sink '{agent_id}': {e}");
                crate::__private::abort();
            }
        };
        self.actions.push((SinkType::Agent(agent_id), action))
    }
}

// TODO: Maybe we rename this trait to "ActionOutput" and remove the concrete type
/// Trait that indicates that a return type can be used as an output of an action function.
///
/// Note: This trait converts `()`, `ActionOutput`, `Result<(), E>` and `Result<ActionOutput, E>` into [`Events`].
/// The implementation of `ActionOutput` also checks, if the writer actually has access to a sink.
pub trait ActionOutEvent: private::Sealed {
    fn convert_out_events(self) -> crate::Result<Events>;
}

mod private {
    pub trait Sealed {}
}

impl private::Sealed for () {}
impl ActionOutEvent for () {
    fn convert_out_events(self) -> crate::Result<Events> {
        Ok(Events::default())
    }
}

impl<E> private::Sealed for Result<(), E> where E: std::fmt::Display + Send + Sync + 'static {}
impl<E> ActionOutEvent for Result<(), E>
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
// AgentCall
// Vec<AgentCall>
//
// .. and their crate::Result<T> equivalents

impl private::Sealed for ActionOutput {}
impl ActionOutEvent for ActionOutput {
    fn convert_out_events(self) -> crate::Result<Events> {
        //let caller = crate::contracts::env::executor();
        //let sinks = crate::contracts::env::sinks();

        //let mut contracts = Vec::new();
        //let mut local = Vec::new();

        //// TODO: There is an edge-case here; we currently have no solution,
        //// if multiple participants in a contract have access to the same sink !
        ////
        //// Idea: Find these places and do a pseudo-random (but deterministic) choice.
        //// Or we could solve this from the outside; somehow..
        //for (sink, action) in self.actions {
        //    match sink {
        //        SinkType::Named(alias) => {
        //            if let Some(sink) = sinks.iter().find(|s| s.has_alias(&alias)) {
        //                if !sink.has_access(caller) {
        //                    debug!("caller {caller} does not have access to sink {alias}");
        //                    continue;
        //                }
        //                match sink {
        //                    Sink::Contract { contract_id, .. } => {
        //                        // TODO
        //                        contracts.push(ContractCall {
        //                            contract_id: *contract_id,
        //                            action,
        //                        })
        //                    }
        //                    Sink::Agent { agent_id, .. } => local.push(AgentCall {
        //                        agent_id: *agent_id,
        //                        action,
        //                    }),
        //                }
        //            } else {
        //                // TODO: Should this be an error or should we just log the error here ?
        //                return Err(anyhow!("Failed to find sink '{alias}', which is referenced in the action output"));
        //            }
        //        }
        //        SinkType::Agent(agent_id) => local.push(AgentCall { agent_id, action }),
        //        // TODO: The edge-case also applies here I guess ??
        //        SinkType::Contract(contract_id) => contracts.push(ContractCall {
        //            contract_id,
        //            action,
        //        }),
        //    }
        //}
        //Ok(Events { contracts, local })
        todo!("re-implement this with the new sink design")
    }
}

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

/// An event Sink for either a contract or sw-agent
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

    ///// Checks weather or not the given user has access to this sink
    //pub fn has_access(&self, user: BorderlessId) -> bool {
    //    match self {
    //        Sink::Agent { owner, .. } => *owner == user,
    //        Sink::Contract {
    //            restrict_to_users, ..
    //        } => {
    //            // If the vector is empty, everyone has access
    //            restrict_to_users.is_empty() || restrict_to_users.iter().any(|u| *u == user)
    //        }
    //    }
    //}

    /// Checks the alias of the sink against some string
    ///
    /// Note: The casing is ignored here, as it should be in all alias lookups.
    pub fn has_alias(&self, alias: impl AsRef<str>) -> bool {
        alias.as_ref().eq_ignore_ascii_case(&self.alias)
    }
}
