use crate::collections::hashmap::proxy::{Entry, Key, Value};
use crate::collections::hashmap::HashMap;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap as StdHashMap;
use std::hash::Hash;

/// Immutable HashMap Iterator
pub struct HashMapIt<'a, K, V> {
    map: &'a HashMap<K, V>,
    idx: usize,
    keys: StdHashMap<u64, ()>,
}

impl<'a, K, V> Iterator for HashMapIt<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    type Item = Entry<'a, K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.keys.iter().nth(self.idx) {
            None => None, // The iterator is consumed
            Some((key, _)) => {
                self.idx = self.idx.saturating_add(1);
                self.map.get_entry(*key)
            }
        }
    }
}

impl<'a, K, V> HashMapIt<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    pub fn new(map: &'a HashMap<K, V>) -> Self {
        // Fetch keys from the HashMap
        let internal_keys = map.internal_keys();

        let mut keys = StdHashMap::with_capacity(internal_keys.len());
        internal_keys.into_iter().for_each(|key| {
            keys.insert(key, ());
        });

        HashMapIt { map, keys, idx: 0 }
    }
}

/// Immutable HashMap Keys Iterator
pub struct Keys<'a, K, V> {
    map: &'a HashMap<K, V>,
    idx: usize,
    keys: StdHashMap<u64, ()>,
}

impl<'a, K, V> Iterator for Keys<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    type Item = Key<'a, K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.keys.iter().nth(self.idx) {
            None => None, // The iterator is consumed
            Some((key, _)) => {
                self.idx = self.idx.saturating_add(1);
                self.map.get_key(*key)
            }
        }
    }
}

impl<'a, K, V> Keys<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    pub fn new(map: &'a HashMap<K, V>) -> Self {
        // Fetch keys from the HashMap
        let internal_keys = map.internal_keys();

        let mut keys = StdHashMap::with_capacity(internal_keys.len());
        internal_keys.into_iter().for_each(|key| {
            keys.insert(key, ());
        });

        Keys { map, keys, idx: 0 }
    }
}

/// Immutable HashMap Values Iterator
pub struct Values<'a, K, V> {
    map: &'a HashMap<K, V>,
    idx: usize,
    keys: StdHashMap<u64, ()>,
}

impl<'a, K, V> Iterator for Values<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    type Item = Value<'a, K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.keys.iter().nth(self.idx) {
            None => None, // The iterator is consumed
            Some((key, _)) => {
                self.idx = self.idx.saturating_add(1);
                self.map.get_value(*key)
            }
        }
    }
}

impl<'a, K, V> Values<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    pub fn new(map: &'a HashMap<K, V>) -> Self {
        // Fetch keys from the HashMap
        let internal_keys = map.internal_keys();

        let mut keys = StdHashMap::with_capacity(internal_keys.len());
        internal_keys.into_iter().for_each(|key| {
            keys.insert(key, ());
        });

        Values { map, keys, idx: 0 }
    }
}
