use crate::__private::registers::REGISTER_CURSOR;
use crate::__private::{read_register, storage_cursor};
use crate::collections::hashmap::proxy::{Key, Proxy};
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
        todo!()
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

pub struct Keys<'a, K, V> {
    map: &'a HashMap<K, V>,
    range: u64,
    global_idx: u64,
}

impl<'a, K, V> Iterator for Keys<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq + Clone,
    V: Serialize + DeserializeOwned,
{
    type Item = Key<'a, K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.global_idx >= self.range {
            // Iterator is consumed
            return None;
        }
        // Read content from register
        let bytes = read_register(REGISTER_CURSOR.saturating_add(self.global_idx))?;
        // Convert into the sub-key
        let arr: [u8; 8] = bytes.as_slice().try_into().expect("Slice length error");
        let sub_key = u64::from_le_bytes(arr);

        let out = self.map.get_key(sub_key);
        self.global_idx = self.global_idx.saturating_add(1);
        out
    }
}

impl<'a, K, V> Keys<'a, K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq + Clone,
    V: Serialize + DeserializeOwned,
{
    pub fn new(map: &'a HashMap<K, V>) -> Self {
        let range = storage_cursor(map.base_key);
        Keys {
            map,
            range,
            global_idx: 0,
        }
    }
}
