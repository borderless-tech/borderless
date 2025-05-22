use std::{
    borrow::Borrow,
    cell::UnsafeCell,
    fmt::Display,
    ops::{Deref, DerefMut},
};

use crate::__private::{
    read_field,
    storage_traits::{private, Storeable, ToPayload},
    write_field,
};
use serde::{de::DeserializeOwned, Serialize};

/// A value that can be read from the storage lazily.
///
/// The value will only be read upon the first access.
/// Afterwards it is cached, while this type keeps track
/// if there are any changes that needs to be synced to disk.
pub struct Lazy<T> {
    value: UnsafeCell<Option<T>>,
    base_key: u64,
    changed: bool,
}

impl<T: Serialize + DeserializeOwned> Lazy<T> {
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
}

impl<T: Serialize + DeserializeOwned> Borrow<T> for Lazy<T> {
    fn borrow(&self) -> &T {
        self.get()
    }
}

impl<T: Serialize + DeserializeOwned> AsRef<T> for Lazy<T> {
    fn as_ref(&self) -> &T {
        self.get()
    }
}

impl<T: Serialize + DeserializeOwned> Deref for Lazy<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T: Serialize + DeserializeOwned> DerefMut for Lazy<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

impl<T: Serialize + DeserializeOwned + Display> Display for Lazy<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.get(), f)
    }
}

impl<T: Serialize + DeserializeOwned + std::fmt::Debug> std::fmt::Debug for Lazy<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stored")
            .field("value", &self.value)
            .field("changed", &self.changed)
            .finish()
    }
}

impl<T: Serialize + DeserializeOwned> private::Sealed for Lazy<T> {}

impl<T: Serialize + DeserializeOwned> Storeable for Lazy<T> {
    fn decode(base_key: u64) -> Self {
        Self {
            value: UnsafeCell::new(None),
            base_key,
            changed: false,
        }
    }

    fn parse_value(value: serde_json::Value, base_key: u64) -> anyhow::Result<Self> {
        let value: T = serde_json::from_value(value)?;
        Ok(Self {
            value: UnsafeCell::new(Some(value)),
            base_key,
            changed: true,
        })
    }

    fn commit(self, base_key: u64) {
        // NOTE: changed is only true, if the value is set and therefore Some(T)
        debug_assert!(self.base_key == base_key);
        if self.changed {
            let value = unsafe { &*self.value.get() };
            write_field(base_key, 0, value.as_ref().unwrap());
        }
    }
}

impl<T: Serialize + DeserializeOwned> ToPayload for Lazy<T> {
    fn to_payload(&self, path: &str) -> anyhow::Result<Option<String>> {
        // Delegate to the inner implementation
        let value = self.get();
        value.to_payload(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Person {
        name: String,
        age: u16,
    }

    fn gen_klaus() -> Person {
        Person {
            name: "Klaus".to_string(),
            age: 42,
        }
    }

    fn make_lazy(p: Person, key: u64) -> Lazy<Person> {
        Lazy {
            value: UnsafeCell::new(Some(p)),
            base_key: key,
            changed: false,
        }
    }

    #[test]
    fn set_get() {
        let p = gen_klaus();
        let mut lazy = make_lazy(p.clone(), 0);
        lazy.set(p.clone());
        assert_eq!(*lazy.get(), p, "get must work");
        assert_eq!(*lazy, p, "deref must work");
    }

    #[test]
    fn deref() {
        let p = gen_klaus();
        let mut lazy = make_lazy(p.clone(), 0);
        lazy.set(p.clone());
        assert_eq!(*lazy, p, "deref must work");
        lazy.age = 0; // deref_mut
        assert_ne!(*lazy, p);
    }

    #[test]
    fn commit_decode() {
        let key = 123456;
        let p = gen_klaus();
        let mut lazy = make_lazy(p.clone(), key);
        lazy.set(p.clone());
        lazy.commit(key);
        let decoded: Lazy<Person> = Lazy::decode(key);
        assert_eq!(*decoded, p);
    }
}
