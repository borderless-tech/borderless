use crate::collections::hashmap::proxy::{Key, Proxy, Value};
use crate::collections::hashmap::HashMap;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::hash::Hash;

/// Immutable HashMap Iterator
pub struct HashMapIt<'a, K, V> {
    map: &'a HashMap<K, V>,
    global_idx: usize,
}

impl<'a, K, V> Iterator for HashMapIt<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    type Item = Proxy<'a, K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

impl<'a, K, V> HashMapIt<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    pub fn new(map: &'a HashMap<K, V>) -> Self {
        HashMapIt { map, global_idx: 0 }
    }
}

/// Immutable HashMap Keys Iterator
pub struct Keys<'a, K, V> {
    map: &'a HashMap<K, V>,
    global_idx: usize,
}

impl<'a, K, V> Iterator for Keys<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    type Item = Key<'a, K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        // Returns None if the iterator is consumed
        let out = self.map.get_key(self.global_idx);
        self.global_idx = self.global_idx.saturating_add(1);
        out
    }
}

impl<'a, K, V> Keys<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    pub fn new(map: &'a HashMap<K, V>) -> Self {
        Keys { map, global_idx: 0 }
    }
}

/// Immutable HashMap Values Iterator
pub struct Values<'a, K, V> {
    map: &'a HashMap<K, V>,
    global_idx: usize,
}

impl<'a, K, V> Iterator for Values<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    type Item = Value<'a, K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        // Returns None if the iterator is consumed
        let out = self.map.get_value(self.global_idx);
        self.global_idx = self.global_idx.saturating_add(1);
        out
    }
}

impl<'a, K, V> Values<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    pub fn new(map: &'a HashMap<K, V>) -> Self {
        Values { map, global_idx: 0 }
    }
}
