use crate::__private::{read_field, storage_has_key, storage_remove, write_field};
use nohash_hasher::IntMap;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use super::ROOT_KEY;
// TODO Store metadata in it?

enum CacheOp {
    Update,
    Remove,
}

pub struct Cache<V> {
    base_key: u64,
    map: RefCell<IntMap<u64, Rc<RefCell<KeyValue<V>>>>>,
    operations: IntMap<u64, CacheOp>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KeyValue<V> {
    key: u64,
    pub(crate) value: V, // Proxy needs access to the field
}

impl<V> KeyValue<V>
where
    V: Serialize + for<'de> Deserialize<'de>,
{
    pub(crate) fn new(key: u64, value: V) -> Self {
        KeyValue { key, value }
    }
}

impl<V> Cache<V>
where
    V: Serialize + for<'de> Deserialize<'de>,
{
    pub(crate) fn new(base_key: u64) -> Self {
        Cache {
            base_key,
            map: RefCell::default(),
            operations: IntMap::default(),
        }
    }

    pub(crate) fn exists(&self) -> bool {
        storage_has_key(self.base_key, ROOT_KEY)
    }

    pub(crate) fn contains_key(&self, sub_key: u64) -> bool {
        // Check first the in-memory copy
        if self.map.borrow().contains_key(&sub_key) {
            return true;
        }
        // Fallback to DB
        storage_has_key(self.base_key, sub_key)
    }

    pub(crate) fn reset(&mut self) {
        // Clear the in-memory map, deallocating the used resources
        self.map = RefCell::default();
        self.operations = IntMap::default();
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

    pub(crate) fn remove(&mut self, key: u64) {
        self.operations.insert(key, CacheOp::Remove);
        self.map.borrow_mut().remove(&key);
    }

    pub(crate) fn flag_write(&mut self, key: u64) {
        self.operations.insert(key, CacheOp::Update);
    }

    pub(crate) fn write(&mut self, key: u64, node: KeyValue<V>) {
        self.operations.insert(key, CacheOp::Update);
        self.map
            .borrow_mut()
            .insert(key, Rc::new(RefCell::new(node)));
    }

    pub(crate) fn clear(&mut self) {
        // Flag all cells for deletion
        for key in self.map.borrow().keys() {
            self.operations.insert(*key, CacheOp::Remove);
        }
        self.map.borrow_mut().clear();

        // TODO Update metadata?
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
    }
}
