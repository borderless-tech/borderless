use std::cell::RefCell;
use std::rc::Rc;

use super::cache::CacheOp::{Remove, Update};
use super::node::Node;
use super::{ORDER, ROOT_KEY};
use nohash_hasher::IntMap;
use serde::{Deserialize, Serialize};

use crate::__private::{
    read_field, storage_gen_sub_key, storage_has_key, storage_remove, write_field,
};

enum CacheOp {
    Update,
    Remove,
}

pub struct Cache<V> {
    base_key: u64,
    map: RefCell<IntMap<u64, Rc<RefCell<Node<V>>>>>,
    operations: IntMap<u64, CacheOp>,
}

impl<V> Cache<V>
where
    V: Serialize + for<'de> Deserialize<'de> + Clone,
{
    pub(crate) fn new(base_key: u64, init: bool) -> Self {
        let mut cache = Cache {
            base_key,
            map: RefCell::default(),
            operations: IntMap::default(),
        };
        if init {
            cache.init();
        }
        cache
    }

    pub(crate) fn exists(&self) -> bool {
        storage_has_key(self.base_key, ROOT_KEY)
    }

    pub(crate) fn init(&mut self) {
        // Create an empty root node, and insert it into the in-memory mirror
        self.write(ROOT_KEY, Node::<V>::empty_leaf(ORDER));
    }

    pub(crate) fn reset(&mut self) {
        // Clear the in-memory map, deallocating the used resources
        self.map = RefCell::default();
        self.operations = IntMap::default();
    }

    pub(crate) fn new_key(&mut self) -> u64 {
        storage_gen_sub_key()
    }

    // TODO
    // pub(crate) fn get_mut(&mut self, key: u64, index: usize) -> Option<&mut V> {
    //     if let None = self.map.get(&key) {
    //         // Add the node to the in-memory mirror
    //         let node = read_field::<Node<V>>(self.base_key, key).unwrap();
    //         self.map.insert(key, Rc::new(node));
    //     }
    //     let leaf = self.map.get_mut(&key).unwrap();
    //     leaf.values.get_mut(index)
    // }

    pub(crate) fn read(&self, key: u64) -> Rc<RefCell<Node<V>>> {
        if let Some(node) = self.map.borrow().get(&key) {
            return node.clone();
        };
        // Add the node to the in-memory mirror
        let node = read_field::<Node<V>>(self.base_key, key).unwrap();
        let node = Rc::new(RefCell::new(node));
        self.map.borrow_mut().insert(key, node.clone());
        node
    }

    pub(crate) fn remove(&mut self, key: u64) {
        self.operations.insert(key, Remove);
        self.map.borrow_mut().remove(&key);
    }

    pub(crate) fn flag_write(&mut self, key: u64) {
        self.operations.insert(key, Update);
    }

    /// Writes a new node to the cache
    pub(crate) fn write(&mut self, key: u64, node: Node<V>) {
        self.operations.insert(key, Update);
        self.map
            .borrow_mut()
            .insert(key, Rc::new(RefCell::new(node)));
    }

    pub(crate) fn clear(&mut self) {
        // Flag all nodes for deletion
        for key in self.map.borrow().keys() {
            self.operations.insert(*key, Remove);
        }
        self.map.borrow_mut().clear();
        self.init();
    }

    pub(crate) fn commit(self) {
        // Sync the in-memory mirror with the DB state
        for (key, op) in &self.operations {
            match op {
                Update => {
                    let map = self.map.borrow();
                    let node = map.get(key).expect("Cache corruption");
                    write_field(self.base_key, *key, node.as_ref());
                }
                Remove => storage_remove(self.base_key, *key),
            }
        }
        // Clears and deallocates the used resources    // TODO Make sure we really want to move self
        //self.operations = IntMap::default();
    }
}
