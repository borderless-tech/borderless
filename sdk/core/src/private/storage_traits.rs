//! Storage trait used to save the state of a contract

use serde::{de::DeserializeOwned, Serialize};
use serde_json::from_value;

use crate::{
    serialize::Value,
    Result,
    __private::{abort, read_field, write_field},
    error,
};

use super::storage_has_key;

pub(crate) mod private {
    /// Seals the implementation of the traits defined in `super`
    pub trait Sealed {}
}

// TODO: Maybe make the decode fallible ?
/// Trait used by the macro for storing and commiting values.
///
/// It is automatically implemented for all types that are serializable by serde.
pub trait Storeable: private::Sealed + Sized {
    /// Decodes a value from the storage using its base-key
    fn decode(base_key: u64) -> Self;

    fn parse_value(value: Value, base_key: u64) -> Result<Self>;

    /// Commits a value to the storage under the given base-key
    fn commit(self, base_key: u64);

    /// Checks, if the value exists in the storage
    ///
    /// Assumes that a value must live under `sub-key=0` !
    fn exists(base_key: u64) -> bool {
        storage_has_key(base_key, 0)
    }
}

/// Indicates a storeable state for either contracts or software-agents.
///
/// Note: You never want to implement this on your own, you always want to derive this trait
pub trait State: Sized {
    /// Loads the state
    fn load() -> Result<Self>;

    /// Initializes the state using a json value
    fn init(value: serde_json::Value) -> Result<Self>;

    /// Use a GET request to query the state
    fn http_get(path: String) -> Result<Option<String>>;

    /// Commit the value to disk
    fn commit(self);

    /// Return the static list of symbols (field-names and their addresses)
    fn symbols() -> &'static [(&'static str, u64)];
}

/// Indicates, that a stored value can be converted into a payload of an http-get request based on the path.
///
/// Note: The payload will be json encoded.
pub trait ToPayload: private::Sealed {
    fn to_payload(&self, path: &str) -> Result<Option<String>>;
}

// This prevents users from implementing their own version of the Storeable trait
impl<T: Serialize + DeserializeOwned> private::Sealed for T {}

// Auto-Impl for serde types
impl<T: Serialize + DeserializeOwned> Storeable for T {
    fn decode(base_key: u64) -> Self {
        // Just read the value from the base-key
        match read_field(base_key, 0) {
            Some(val) => val,
            None => {
                error!("Failed to decode stored value: Base-Key {base_key} not found !");
                abort();
            }
        }
    }

    fn parse_value(value: Value, _base_key: u64) -> Result<Self> {
        Ok(from_value(value)?)
    }

    fn commit(self, base_key: u64) {
        // Just commit the value to the base-key
        write_field(base_key, 0, &self)
    }
}

impl<T: Serialize + private::Sealed> ToPayload for T {
    fn to_payload(&self, path: &str) -> Result<Option<String>> {
        // Different Approach:
        let value = serde_json::to_value(self)?;

        // Instantly return the value
        if path.is_empty() {
            match value {
                Value::String(s) => return Ok(Some(s)),
                other => return Ok(Some(other.to_string())),
            }
        }

        // Search sub-fields based on path
        let mut current = &value;
        for seg in path
            .split('/')
            .flat_map(|s| if s.is_empty() { None } else { Some(s) })
        {
            current = match current.get(seg) {
                Some(v) => v,
                None => return Ok(None),
            };
        }
        match current {
            Value::String(s) => Ok(Some(s.clone())),
            other => Ok(Some(other.to_string())),
        }
    }
}
