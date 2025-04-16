//! Storage trait used to save the state of a contract

use serde::{de::DeserializeOwned, Serialize};
use serde_json::from_value;

use crate::{
    serialize::Value,
    Result,
    __private::{abort, read_field, write_field},
    error,
};

// TODO: Maybe make the decode fallible ?
/// Trait used by the macro for storing and commiting values.
///
/// It is automatically implemented for all types that are serializable by serde (as they have a [`Packed`] layout).
pub trait Storeable: private::Sealed + Sized {
    /// Decodes a value from the storage using its base-key
    fn decode(base_key: u64) -> Self;

    fn parse_value(value: Value, base_key: u64) -> Result<Self>;

    /// Commits a value to the storage under the given base-key
    fn commit(self, base_key: u64);
}

pub(crate) mod private {
    /// Seals the implementation of `Packed` and `Storeable`.
    pub trait Sealed {}
}

// TODO: Do I really need the "packed" trait, or is this boilerplate ?

/// Trait the implies a "packed" layout
///
/// This is automatically implemented for all types that implement [`serde::Serialize`] and [`serde::Deserialize`].
/// Those types will be stored under a single key by simply serializing them with [`postcard`].
/// There are no sub-keys attached to these types.
pub trait Packed: Storeable {}

// This prevents users from implementing their own version of the Storeable trait
impl<T: Serialize + DeserializeOwned> private::Sealed for T {}

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

// Serde types are automatically packed
impl<T: Serialize + DeserializeOwned> Packed for T {}

/// Indicates a storeable state for either contracts or software-agents.
///
/// Note: You never want to implement this on your own, you always want to derive this trait
pub trait State: Sized {
    fn load() -> Result<Self>;

    fn init(value: serde_json::Value) -> Result<Self>;

    fn http_get(path: String) -> Option<String>;

    fn commit();
}
