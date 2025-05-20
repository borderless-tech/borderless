use serde::{Deserialize, Serialize};

use crate::{
    __private::send_ws_msg,
    events::{ActionOutput, CallAction, MethodOrId},
};

// TODO: Are schedules completely static ?
// Or do we want to enable temporary schedules,
// that can be registered on runtime by other schedules ?
/// Schedules are functions that are executed periodically.
///
/// This struct is the equivalent of [`CallAction`], just for schedules.
/// Internally all schedules are just actions without any input parameters.
/// They are executed by injecting a [`CallAction`] object with the correct method name and an empty `params` field.
///
/// Like `CallAction` the `Schedule` struct is always serialized and deserialized as json.
#[derive(Debug, Serialize, Deserialize)]
pub struct Schedule {
    /// Method that is called periodically
    #[serde(flatten)]
    pub method: MethodOrId,
    /// Schedule period in milliseconds
    pub period: u64,
    /// Delay in milliseconds for the first schedule execution. Defaults to `0`.
    #[serde(default)]
    pub delay: u64,
}

impl Schedule {
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    pub fn get_action(&self) -> CallAction {
        CallAction::new(self.method.clone(), serde_json::Value::Null)
    }
}

/// Capabilities of a SW-Agent
#[derive(Serialize, Deserialize)]
pub struct Capabilities {
    /// Weather or not the agent is allowed to make http-calls
    pub network: bool,
    /// Weather or not the agent is allowed to establish websocket connections
    pub websocket: bool,
    /// URLs that the agent is allowed to call
    pub url_whitelist: Vec<String>,
}

/// Websocket configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsConfig {
    /// Websocket URL
    pub url: String,

    /// Weather or not we will automatically reconnect if a connection was closed
    pub reconnect: bool,

    /// Time interval in seconds for the `Ping` messages
    ///
    /// Can be configured to handle different proxy timeout configurations.
    pub ping_interval: u64,

    /// Weather or not the messages over this channel are binary or text
    #[serde(default)]
    pub binary: bool,
}
// NOTE: We could maybe wrap the WsConfig in an AdvancedConfig object,
// which contains also the addresses of the WebsocketHandler functions ?

pub trait WebsocketHandler {
    type Err: std::fmt::Display + std::fmt::Debug;

    /// Constructor function that is called before the connection is opened.
    ///
    /// This function returns all required information to establish the websocket connection.
    fn open_ws() -> WsConfig;

    /// Called when a new connection is established (before any messages are exchanged).
    fn on_open(&mut self) -> Result<Option<ActionOutput>, Self::Err>;

    /// Called whenever a message is received from the client.
    fn on_message(&mut self, msg: Vec<u8>) -> Result<Option<ActionOutput>, Self::Err>;

    /// Called when an error occurs on the connection.
    fn on_error(&mut self) -> Result<Option<ActionOutput>, Self::Err>;

    /// Called when the connection is cleanly closed (e.g., by the client).
    fn on_close(&mut self, code: u16, reason: &str) -> Result<Option<ActionOutput>, Self::Err>;

    /// Send a message to the other side
    fn send_msg(&self, msg: Vec<u8>) -> Result<(), anyhow::Error> {
        send_ws_msg(msg)
    }
}

/// Return value of the init function
#[derive(Debug, Serialize, Deserialize)]
pub struct Init {
    pub schedules: Vec<Schedule>,
    pub ws_config: Option<WsConfig>,
}

impl Init {
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}
