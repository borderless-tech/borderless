use crate::collections::hashmap::proxy::Proxy;
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
    K: Serialize + DeserializeOwned + Hash + Eq + Clone,
    V: Serialize + DeserializeOwned,
{
    type Item = Proxy<'a, K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        let out = self.map.at(self.global_idx);
        self.global_idx = self.global_idx.saturating_add(1);
        out
    }
}

impl<'a, K, V> HashMapIt<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq + Clone,
    V: Serialize + DeserializeOwned,
{
    pub fn new(map: &'a HashMap<K, V>) -> Self {
        HashMapIt { map, global_idx: 0 }
    }
}
