use std::{
    borrow::Borrow,
    cell::UnsafeCell,
    fmt::Display,
    ops::{Deref, DerefMut},
};

pub use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{read_field, storage_has_key, write_field};
pub mod error {
    pub use anyhow::{Context, Error};
}

// NOTE: We require some special keys for contract-info, contract-desc, contract-metadata and so on

/// Trait that indicates, that a type / field can be stored into our key-value-storage.
pub trait Storable {
    /// Stores the current value to the key-value store
    fn store(&self);

    /// Returns the key of the stored value
    fn key(&self) -> u64;

    /// Checks, if a value exists for our key
    fn exists(&self) -> bool;
}

/// Marker tait that indicates, that the struct only consists of fields, that are storeable.
pub trait StorableType {
    fn exists(&self) -> bool;

    fn commit(&self);
}

// Maybe create storage keys like this: [ u32; u32 ]
// Where the first u64 is for the field identifier and the second one is for elements
// that also belong to that identifier (e.g. HashMaps, vectors etc.).
// I think in general for the fields, 32 bit would be enough,
// so maybe we can even split this up in [ u64, u64, u64 ] ?
//
// -> Maybe we completely f this, and use a u64 here, because collisions in u64 space are super unlikely.
// This would allow us to handle nested datastructures, as they have an almost 0% chance of collision with the
// fields of the outer datatype, that holds the nested structure.

/// A value that can be read from the storage
///
/// The value will only be read upon the first access.
/// Afterwards it is cached, while this type keeps track
/// if there are any changes that needs to be synced to disk.
pub struct Stored<T, const KEY: u64> {
    value: UnsafeCell<Option<T>>,
    changed: bool,
}

impl<T, const KEY: u64> Default for Stored<T, KEY> {
    fn default() -> Self {
        Self {
            value: UnsafeCell::new(None),
            changed: false,
        }
    }
}

impl<T: Serialize + DeserializeOwned, const KEY: u64> Stored<T, KEY> {
    pub fn init(value: T) -> Self {
        Self {
            value: UnsafeCell::new(Some(value)),
            changed: true,
        }
    }

    pub fn get(&self) -> &T {
        let value = unsafe { &mut *self.value.get() };

        if let Some(ref val) = *value {
            return val;
        }
        let new_value = read_field(self.key(), 0).unwrap();
        *value = Some(new_value);

        // SAFETY: After setting, it is safe to return an immutable reference to the inner value.
        if let Some(ref val) = *value {
            val
        } else {
            unreachable!("We just set the value, so this should never happen.");
        }
    }

    pub fn get_mut(&mut self) -> &mut T {
        // If there is a mutable borrow, there will be a change.
        self.changed = true;
        let value = unsafe { &mut *self.value.get() };

        if let Some(ref mut val) = *value {
            return val;
        }
        let new_value = read_field(self.key(), 0).unwrap();
        *value = Some(new_value);

        // SAFETY: After setting, it is safe to return an immutable reference to the inner value.
        if let Some(ref mut val) = *value {
            val
        } else {
            unreachable!("We just set the value, so this should never happen.");
        }
    }

    pub fn set(&mut self, value: T) {
        self.changed = true;
        let old_value = unsafe { &mut *self.value.get() };
        *old_value = Some(value);
    }
}

impl<T: Serialize, const KEY: u64> Storable for Stored<T, KEY> {
    fn store(&self) {
        // NOTE: changed is only true, if the value is set and therefore Some(T)
        if self.changed {
            let value = unsafe { &*self.value.get() };
            write_field(self.key(), 0, value.as_ref().unwrap());
        }
    }

    fn key(&self) -> u64 {
        KEY
    }

    fn exists(&self) -> bool {
        storage_has_key(self.key(), 0)
    }
}

impl<T: Serialize + DeserializeOwned, const KEY: u64> Borrow<T> for Stored<T, KEY> {
    fn borrow(&self) -> &T {
        self.get()
    }
}

impl<T: Serialize + DeserializeOwned, const KEY: u64> AsRef<T> for Stored<T, KEY> {
    fn as_ref(&self) -> &T {
        self.get()
    }
}

impl<T: Serialize + DeserializeOwned, const KEY: u64> Deref for Stored<T, KEY> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T: Serialize + DeserializeOwned, const KEY: u64> DerefMut for Stored<T, KEY> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

impl<T: Serialize + DeserializeOwned, const KEY: u64> From<T> for Stored<T, KEY> {
    fn from(value: T) -> Self {
        Self::init(value)
    }
}

impl<T: Serialize + DeserializeOwned + Display, const KEY: u64> Display for Stored<T, KEY> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.get(), f)
    }
}

impl<T: Serialize + DeserializeOwned + std::fmt::Debug, const KEY: u64> std::fmt::Debug
    for Stored<T, KEY>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stored")
            .field("value", &self.value)
            .field("changed", &self.changed)
            .finish()
    }
}

impl<T: Serialize + DeserializeOwned, const KEY: u64> Serialize for Stored<T, KEY> {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        T::serialize(&self, serializer)
    }
}

impl<'de, T: Deserialize<'de>, const KEY: u64> Deserialize<'de> for Stored<T, KEY> {
    fn deserialize<D>(deserializer: D) -> std::prelude::v1::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(|ok| Stored {
            value: UnsafeCell::new(Some(ok)),
            changed: false,
        })
    }
}
