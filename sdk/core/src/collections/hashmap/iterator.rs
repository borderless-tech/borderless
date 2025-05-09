use crate::collections::hashmap::proxy::{Entry, Key, Value};
use crate::collections::hashmap::HashMap;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashSet;
use std::hash::Hash;

/// Immutable HashMap Iterator
pub struct HashMapIt<'a, K, V> {
    map: &'a HashMap<K, V>,
    idx: usize,
    keys: HashSet<u64>,
}

impl<'a, K, V> Iterator for HashMapIt<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    type Item = Entry<'a, K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        self.keys.iter().nth(self.idx).and_then(|key| {
            self.idx = self.idx.saturating_add(1);
            self.map.get_entry(*key)
        })
    }
}

impl<'a, K, V> HashMapIt<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    pub fn new(map: &'a HashMap<K, V>) -> Self {
        // Fetch keys from the HashMap
        let keys = map.internal_keys();
        HashMapIt { map, keys, idx: 0 }
    }
}

/// Immutable HashMap Keys Iterator
pub struct Keys<'a, K, V> {
    map: &'a HashMap<K, V>,
    idx: usize,
    keys: HashSet<u64>,
}

impl<'a, K, V> Iterator for Keys<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    type Item = Key<'a, K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        self.keys.iter().nth(self.idx).and_then(|key| {
            self.idx = self.idx.saturating_add(1);
            self.map.get_key(*key)
        })
    }
}

impl<'a, K, V> Keys<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    pub fn new(map: &'a HashMap<K, V>) -> Self {
        // Fetch keys from the HashMap
        let keys = map.internal_keys();
        Keys { map, keys, idx: 0 }
    }
}

/// Immutable HashMap Values Iterator
pub struct Values<'a, K, V> {
    map: &'a HashMap<K, V>,
    idx: usize,
    keys: HashSet<u64>,
}

impl<'a, K, V> Iterator for Values<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    type Item = Value<'a, K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        self.keys.iter().nth(self.idx).and_then(|key| {
            self.idx = self.idx.saturating_add(1);
            self.map.get_value(*key)
        })
    }
}

impl<'a, K, V> Values<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    pub fn new(map: &'a HashMap<K, V>) -> Self {
        // Fetch keys from the HashMap
        let keys = map.internal_keys();
        Values { map, keys, idx: 0 }
    }
}
