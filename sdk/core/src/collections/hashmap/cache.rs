use super::ROOT_KEY;
use crate::__private::{read_field, storage_has_key, storage_remove, write_field};
use crate::collections::hashmap::metadata::Metadata;
use nohash_hasher::IntMap;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

enum CacheOp {
    Update,
    Remove,
}

pub struct Cache<V> {
    base_key: u64,
    map: RefCell<IntMap<u64, Rc<RefCell<KeyValue<V>>>>>,
    operations: IntMap<u64, CacheOp>,
    metadata: Metadata<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KeyValue<V> {
    pub(crate) key: u64,
    pub(crate) value: V, // Proxy needs access to the field
}

impl<V> KeyValue<V>
where
    V: Serialize + DeserializeOwned,
{
    pub(crate) fn new(key: u64, value: V) -> Self {
        KeyValue { key, value }
    }
}

impl<V> Cache<V>
where
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

    pub(crate) fn len(&self) -> usize {
        self.metadata.len()
    }

    pub(crate) fn keys(&self) -> Vec<u64> {
        self.metadata.keys()
    }

    pub(crate) fn exists(&self) -> bool {
        storage_has_key(self.base_key, ROOT_KEY)
    }

    pub(crate) fn contains_key(&self, sub_key: u64) -> bool {
        self.read(sub_key).is_some()
    }

    pub(crate) fn read(&self, key: u64) -> Option<Rc<RefCell<KeyValue<V>>>> {
        // Check first the in-memory copy
        if let Some(cell) = self.map.borrow().get(&key) {
            return Some(cell.clone());
        }
        // Fallback to DB
        match read_field::<KeyValue<V>>(self.base_key, key) {
            None => None,
            Some(keypair) => {
                let cell = Rc::new(RefCell::new(keypair));
                // Add value to the in-memory mirror
                self.map.borrow_mut().insert(key, cell.clone());
                Some(cell)
            }
        }
    }

    pub(crate) fn remove(&mut self, key: u64) -> Option<V> {
        // Flag key as removed
        self.operations.insert(key, CacheOp::Remove);
        // Remove value from cache
        let old_rc = self.map.borrow_mut().remove(&key);
        // Handle old value
        match old_rc {
            None => None,
            Some(old_rc) => {
                let old_cell = Rc::try_unwrap(old_rc).ok().expect("Rc strong counter > 1");
                Some(old_cell.into_inner().value)
            }
        }
    }

    pub(crate) fn flag_write(&mut self, key: u64) {
        self.operations.insert(key, CacheOp::Update);
    }

    pub(crate) fn insert(&mut self, key: u64, node: KeyValue<V>) -> Option<V> {
        // Flag key as modified
        self.operations.insert(key, CacheOp::Update);
        // Insert new value
        let old_rc = self
            .map
            .borrow_mut()
            .insert(key, Rc::new(RefCell::new(node)));
        // Handle old value
        match old_rc {
            None => None,
            Some(old_rc) => {
                let old_cell = Rc::try_unwrap(old_rc).ok().expect("Rc strong counter > 1");
                Some(old_cell.into_inner().value)
            }
        }
    }

    pub(crate) fn clear(&mut self) {
        // Clear the in-memory map, deallocating the used resources
        self.map = RefCell::default();
        self.operations = IntMap::default();

        // Flag all cells for deletion
        for key in self.metadata.keys() {
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
}
