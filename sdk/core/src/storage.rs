use std::{
    borrow::Borrow,
    cell::UnsafeCell,
    fmt::Display,
    ops::{Deref, DerefMut},
};

use crate::{read_field, storage_has_key, write_field};
use serde::{de::DeserializeOwned, Serialize};

/// A value that can be read from the storage
///
/// The value will only be read upon the first access.
/// Afterwards it is cached, while this type keeps track
/// if there are any changes that needs to be synced to disk.
pub struct Stored<T> {
    value: UnsafeCell<Option<T>>,
    base_key: u32,
    changed: bool,
}

impl<T: Serialize + DeserializeOwned> Stored<T> {
    pub fn new(base_key: u32, value: Option<T>) -> Self {
        let changed = value.is_some();
        Self {
            value: UnsafeCell::new(value),
            base_key,
            changed,
        }
    }
    pub fn init(base_key: u32, value: T) -> Self {
        Self::new(base_key, Some(value))
    }

    pub fn open(base_key: u32) -> Self {
        Self::new(base_key, None)
    }

    pub fn get(&self) -> &T {
        let value = unsafe { &mut *self.value.get() };

        if let Some(ref val) = *value {
            return val;
        }
        let new_value = read_field(self.base_key, 0).unwrap();
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
        let new_value = read_field(self.base_key, 0).unwrap();
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

    // TODO: This has to be used in some kind of sealed trait
    pub fn store(&self) {
        // NOTE: changed is only true, if the value is set and therefore Some(T)
        if self.changed {
            let value = unsafe { &*self.value.get() };
            write_field(self.base_key, 0, value.as_ref().unwrap());
        }
    }

    // TODO: This has to be used in some kind of sealed trait
    pub fn exists(&self) -> bool {
        storage_has_key(self.base_key, 0)
    }
}

impl<T: Serialize + DeserializeOwned> Borrow<T> for Stored<T> {
    fn borrow(&self) -> &T {
        self.get()
    }
}

impl<T: Serialize + DeserializeOwned> AsRef<T> for Stored<T> {
    fn as_ref(&self) -> &T {
        self.get()
    }
}

impl<T: Serialize + DeserializeOwned> Deref for Stored<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T: Serialize + DeserializeOwned> DerefMut for Stored<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

// impl<T: Serialize + DeserializeOwned> From<T> for Stored<T> {
//     fn from(value: T) -> Self {
//         Self::init(value)
//     }
// }

impl<T: Serialize + DeserializeOwned + Display> Display for Stored<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.get(), f)
    }
}

impl<T: Serialize + DeserializeOwned + std::fmt::Debug> std::fmt::Debug for Stored<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stored")
            .field("value", &self.value)
            .field("changed", &self.changed)
            .finish()
    }
}

impl<T: Serialize + DeserializeOwned> Serialize for Stored<T> {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        T::serialize(&self, serializer)
    }
}

// impl<'de, T: Deserialize<'de>> Deserialize<'de> for Stored<T> {
//     fn deserialize<D>(deserializer: D) -> std::prelude::v1::Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         T::deserialize(deserializer).map(|ok| Stored {
//             value: UnsafeCell::new(Some(ok)),
//             changed: false,
//         })
//     }
// }
