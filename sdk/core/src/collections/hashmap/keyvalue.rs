use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[derive(Serialize, Deserialize)]
pub struct KeyValue<K, V> {
    pub(crate) pair: (K, V), // Proxy needs access to the field
}

impl<K, V> KeyValue<K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    pub(crate) fn new(key: K, value: V) -> Self {
        KeyValue { pair: (key, value) }
    }
}
