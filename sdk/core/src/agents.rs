use serde::{Deserialize, Serialize};

use crate::events::{CallAction, MethodOrId};

/// Schedules are functions that are executed periodically.
///
/// This struct is the equivalent of [`CallAction`], just for schedules.
/// Internally all schedules are just actions without any input parameters.
/// They are executed by injecting a [`CallAction`] object with the correct method name and an empty `params` field.
///
/// Like `CallAction` the `Schedule` struct is always serialized and deserialized as json.
#[derive(Serialize, Deserialize)]
pub struct Schedule {
    /// Method that is called periodically
    #[serde(flatten)]
    pub method: MethodOrId,
    /// Schedule period in seconds
    pub period: u32,
    /// Delay in seconds for the first schedule execution. Has no meaning if `immediate=true`
    #[serde(default)]
    pub delay: u32,
    /// Weather or not the schedule should be executed immediately after a sw-agent has started
    #[serde(default)]
    pub immediate: bool,
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
