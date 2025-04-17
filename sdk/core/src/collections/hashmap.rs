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
use crate::collections::lazyvec::ROOT_KEY;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cell::RefCell;
use std::marker::PhantomData;

use cache::{Cache, Cell};
use proxy::Proxy;

pub struct HashMap<V> {
    cache: Cache<V>,
}

impl<V> Sealed for HashMap<V> {}

impl<V> storage_traits::Storeable for HashMap<V> {
    fn decode(base_key: u64) -> Self {
        todo!()
    }

    fn parse_value(value: Value, base_key: u64) -> anyhow::Result<Self> {
        todo!()
    }

    fn commit(self, _base_key: u64) {
        todo!()
    }
}

impl<V> HashMap<V>
where
    V: Serialize + for<'de> Deserialize<'de>,
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

    pub fn contains(&self, value: V) -> bool {
        todo!()
    }

    pub fn insert(&mut self, key: u64, value: V) -> Option<Proxy<'_, V>> {
        //let prev = self.get(key);
        self.cache.write(key, Cell::new(key, value));
        //prev
        None // TODO Fix borrow conflicts
    }

    pub fn remove(&mut self, key: u64) -> Option<V> {
        todo!()
    }

    pub fn get(&self, key: u64) -> Option<Proxy<'_, V>> {
        let cell = self.cache.read(key);
        let proxy = Proxy {
            value_ptr: RefCell::new(cell),
            _back_ref: PhantomData,
        };
        Some(proxy)
    }

    // TODO Implement the following methods
    // get_mut()

    // These methods convert our Lazy Hashmap into a regular hashmap
    // keys()
    // value()

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

    // Fetches all the nodes from the DB, loading them in the cache
    fn load(&mut self, key: u64) {
        todo!()
    }
}
