/*
 * IntMap<u64, Product>;
 *         |      |
 *      sub-key   +-> ( key<u64>, value<Product> )
 *
 * get(key: u64)                    -> read_field(BASE_KEY, key) -> (key, value) -> &(_, value)
 * insert(key: u64, value: Product) -> (key, value) -> write_field(BASE_KEY, key, value)
 *
 * Map<String, Product>
 *         |      |
 *      sub-key   +-> ( key<String>, value<Product> )
 */
mod cache;
mod metadata;
mod proxy;

use super::lazyvec::proxy::Proxy as LazyVecProxy;
use crate::__private::storage_traits;
use crate::__private::storage_traits::private::Sealed;
use cache::{Cache, KeyValue};
use proxy::{Proxy, ProxyMut};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;

pub struct HashMap<K, V> {
    cache: Cache<K, V>,
}

impl<K, V> Debug for HashMap<K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq + Clone + Debug,
    V: Serialize + DeserializeOwned + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{{")?;
        for key in self.keys() {
            if let Some(keypair) = self.get((*key).clone()) {
                let cell = keypair.cell_ptr.borrow();
                writeln!(f, "    {:?}: {:?},", cell.key, cell.value)?;
            }
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

impl<K, V> Sealed for HashMap<K, V> {}

impl<K, V> storage_traits::Storeable for HashMap<K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq + Clone,
    V: Serialize + DeserializeOwned,
{
    fn decode(base_key: u64) -> Self {
        Self::open(base_key)
    }

    fn parse_value(value: Value, base_key: u64) -> anyhow::Result<Self> {
        let values: Vec<KeyValue<K, V>> = serde_json::from_value(value)?;
        let mut out = Self::new(base_key);
        for v in values {
            let key = v.key;
            let value = v.value;
            out.insert(key, value);
        }
        Ok(out)
    }

    fn commit(self, _base_key: u64) {
        self.cache.commit()
    }
}

impl<K, V> HashMap<K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq + Clone,
    V: Serialize + DeserializeOwned,
{
    pub(crate) fn new(base_key: u64) -> Self {
        HashMap {
            cache: Cache::new(base_key),
        }
    }

    pub(crate) fn open(base_key: u64) -> Self {
        HashMap {
            cache: Cache::open(base_key),
        }
    }

    pub fn exists(&self) -> bool {
        self.cache.exists()
    }

    pub fn len_at_shard(&self, index: usize) -> usize {
        self.cache.len_at_shard(index)
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let kv = KeyValue::new(key.clone(), value);
        self.cache.insert(key, kv)
    }

    pub fn remove(&mut self, key: K) -> Option<V> {
        self.cache.remove(key)
    }

    pub fn get(&self, key: K) -> Option<Proxy<'_, K, V>> {
        self.cache.get(key)
    }

    pub fn get_mut(&mut self, key: K) -> Option<ProxyMut<'_, K, V>> {
        self.cache.get_mut(key)
    }

    pub fn contains_key(&self, key: K) -> bool {
        self.cache.contains_key(key)
    }

    pub fn clear(&mut self) {
        // Discard the local changes and clear its content
        self.cache.clear();
    }

    pub fn keys(&self) -> impl Iterator<Item = LazyVecProxy<'_, K>> + '_ {
        self.cache.keys()
    }

    pub fn values(&self) {
        todo!()
    }
}
