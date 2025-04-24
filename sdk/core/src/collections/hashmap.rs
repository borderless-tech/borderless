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
mod proxy;

use crate::__private::storage_traits;
use crate::__private::storage_traits::private::Sealed;
use cache::{Cache, KeyValue};
use proxy::{Proxy, ProxyMut};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::marker::PhantomData;

pub(crate) const ROOT_KEY: u64 = 0;

pub struct HashMap<V> {
    cache: Cache<V>,
}

impl<V> Sealed for HashMap<V> {}

impl<V> storage_traits::Storeable for HashMap<V>
where
    V: Serialize + DeserializeOwned,
{
    fn decode(base_key: u64) -> Self {
        todo!()
    }

    fn parse_value(value: Value, base_key: u64) -> anyhow::Result<Self> {
        todo!()
    }

    fn commit(self, _base_key: u64) {
        self.cache.commit()
    }
}

impl<V> HashMap<V>
where
    V: Serialize + DeserializeOwned,
{
    pub fn new(base_key: u64) -> Self {
        HashMap {
            cache: Cache::new(base_key),
        }
    }

    pub fn len(&self) -> usize {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, key: u64, value: V) -> Option<V> {
        let kv = KeyValue::new(key, value);
        self.cache.insert(key, kv)
    }

    pub fn remove(&mut self, key: u64) -> Option<V> {
        self.cache.remove(key)
    }

    pub fn get(&self, key: u64) -> Option<Proxy<'_, V>> {
        match self.cache.read(key) {
            None => None,
            Some(cell) => {
                let proxy = Proxy {
                    cell_ptr: cell,
                    _back_ref: PhantomData,
                };
                Some(proxy)
            }
        }
    }

    pub fn get_mut(&mut self, key: u64) -> Option<ProxyMut<'_, V>> {
        match self.cache.read(key) {
            None => None,
            Some(cell) => {
                // NOTE: Mark the node as changed, because the user could totally do that.
                self.cache.flag_write(key);
                let proxy = ProxyMut {
                    cell_ptr: cell,
                    _back_ref: PhantomData,
                };
                Some(proxy)
            }
        }
    }

    pub fn contains_key(&self, key: u64) -> bool {
        self.cache.contains_key(key)
    }

    pub fn clear(&mut self) {
        // Discard the local changes
        self.cache.reset();
        // Loads all the nodes to the cache
        self.load(ROOT_KEY);
        self.cache.clear();
    }

    // TODO Implement the following methods (they are eager methods instead of lazy)
    // keys()
    // value()

    // Fetches all the nodes from the DB, loading them in the cache
    fn load(&mut self, key: u64) {
        todo!()
    }
}
