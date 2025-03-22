use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// TODO: Add provable json document stuff

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallAction {
    // TODO: use an enum with flattening here (maybe ?)
    pub method: Option<String>,
    pub method_id: Option<u32>,
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
            method: Some(method_name.as_ref().to_string()),
            method_id: None,
            params,
        }
    }

    pub fn by_method_id(method_id: u32, params: Value) -> Self {
        Self {
            method: None,
            method_id: Some(method_id),
            params,
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
