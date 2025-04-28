use super::metadata::SEED;
use crate::__private::{read_field, storage_has_key, storage_remove, write_field};
use crate::collections::hashmap::metadata::Metadata;
use crate::collections::hashmap::proxy::{Proxy, ProxyMut};
use crate::collections::lazyvec::proxy::Proxy as LazyVecProxy;
use nohash_hasher::IntMap;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;
use std::rc::Rc;
use xxhash_rust::xxh64::Xxh64;

enum CacheOp {
    Update,
    Remove,
}

pub struct Cache<K, V> {
    base_key: u64,
    map: RefCell<IntMap<u64, Rc<RefCell<KeyValue<K, V>>>>>,
    operations: IntMap<u64, CacheOp>,
    metadata: Metadata<K>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KeyValue<K, V> {
    pub(crate) key: K,
    pub(crate) value: V, // Proxy needs access to the field
}

impl<K, V> Debug for KeyValue<K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq + Debug,
    V: Serialize + DeserializeOwned + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?}: {:?},", self.key, self.value)
    }
}

impl<K, V> KeyValue<K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    pub(crate) fn new(key: K, value: V) -> Self {
        KeyValue { key, value }
    }
}

impl<K, V> Cache<K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    pub(crate) fn new(base_key: u64) -> Self {
        Cache {
            base_key,
            map: RefCell::default(),
            operations: IntMap::default(),
            metadata: Metadata::new(base_key),
        }
    }

    pub(crate) fn open(base_key: u64) -> Self {
        Cache {
            base_key,
            map: RefCell::default(),
            operations: IntMap::default(),
            metadata: Metadata::open(base_key),
        }
    }

    pub(crate) fn len_at_shard(&self, index: usize) -> usize {
        self.metadata.len_at_shard(index)
    }

    pub(crate) fn len(&self) -> usize {
        self.metadata.len()
    }

    pub(crate) fn keys(&self) -> impl Iterator<Item = LazyVecProxy<'_, K>> + '_ {
        self.metadata.keys()
    }

    pub(crate) fn exists(&self) -> bool {
        // Check if first shard exists
        storage_has_key(self.base_key, 1)
    }

    pub(crate) fn contains_key(&self, sub_key: K) -> bool {
        let sub_key = Self::hash_key(&sub_key);
        self.read(sub_key).is_some()
    }

    pub(crate) fn read(&self, key: u64) -> Option<Rc<RefCell<KeyValue<K, V>>>> {
        // Deleted keys (but still not commited) must return None
        // as the DB still contains them
        if let Some(CacheOp::Remove) = self.operations.get(&key) {
            return None;
        }
        // Check first the in-memory copy
        if let Some(cell) = self.map.borrow().get(&key) {
            return Some(cell.clone());
        }
        // Fallback to DB
        match read_field::<KeyValue<K, V>>(self.base_key, key) {
            None => None,
            Some(keypair) => {
                let cell = Rc::new(RefCell::new(keypair));
                // Add value to the in-memory mirror
                self.map.borrow_mut().insert(key, cell.clone());
                Some(cell)
            }
        }
    }

    pub(crate) fn remove(&mut self, key: K) -> Option<V> {
        let internal_key = Self::hash_key(&key);
        // Check if key is present
        self.read(internal_key)?;
        // Remove key from metadata
        self.metadata.remove(key);
        // Flag key as removed
        self.operations.insert(internal_key, CacheOp::Remove);
        // Remove value from cache
        let mut map = self.map.borrow_mut();
        map.remove(&internal_key).and_then(Self::extract_cell)
    }

    pub(crate) fn flag_write(&mut self, key: u64) {
        self.operations.insert(key, CacheOp::Update);
    }

    pub(crate) fn insert(&mut self, key: K, value: KeyValue<K, V>) -> Option<V> {
        let internal_key = Self::hash_key(&key);
        if self.read(internal_key).is_none() {
            // Add key to metadata
            self.metadata.insert(key);
        }
        // Flag key as modified
        self.operations.insert(internal_key, CacheOp::Update);
        // Insert new value
        let mut map = self.map.borrow_mut();
        let cell = Rc::new(RefCell::new(value));
        map.insert(internal_key, cell).and_then(Self::extract_cell)
    }

    pub(crate) fn get(&self, key: K) -> Option<Proxy<'_, K, V>> {
        let internal_key = Self::hash_key(&key);
        match self.read(internal_key) {
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

    pub fn get_mut(&mut self, key: K) -> Option<ProxyMut<'_, K, V>> {
        let internal_key = Self::hash_key(&key);
        match self.read(internal_key) {
            None => None,
            Some(cell) => {
                // NOTE: Mark the node as changed, because the user could totally do that.
                self.flag_write(internal_key);
                let proxy = ProxyMut {
                    cell_ptr: cell,
                    _back_ref: PhantomData,
                };
                Some(proxy)
            }
        }
    }

    pub(crate) fn clear(&mut self) {
        // Clear the in-memory map, deallocating the used resources
        self.map = RefCell::default();
        self.operations = IntMap::default();

        // Flag all cells for deletion
        for key in self.metadata.keys() {
            let key = Self::hash_key(&key);
            self.operations.insert(key, CacheOp::Remove);
        }
        // Clear metadata
        self.metadata.clear();
    }

    pub(crate) fn commit(self) {
        // Sync the in-memory mirror with the DB state
        for (key, op) in &self.operations {
            match op {
                CacheOp::Update => {
                    let map = self.map.borrow();
                    let cell = map.get(key).expect("Cache corruption");
                    write_field(self.base_key, *key, cell.as_ref());
                }
                CacheOp::Remove => storage_remove(self.base_key, *key),
            }
        }
        // Commit metadata
        self.metadata.commit();
    }

    fn extract_cell(rc: Rc<RefCell<KeyValue<K, V>>>) -> Option<V> {
        let old_cell = Rc::try_unwrap(rc).ok().expect("Rc strong counter > 1");
        Some(old_cell.into_inner().value)
    }

    fn hash_key(key: &K) -> u64 {
        let mut h = Xxh64::new(SEED);
        key.hash(&mut h);
        h.digest()
    }
}
