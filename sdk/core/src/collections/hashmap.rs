mod iterator;
mod keyvalue;
mod proxy;

use super::hashmap::keyvalue::KeyValue;
use super::hashmap::proxy::{
    Entry as EntryProxy, Key as KeyProxy, Value as ValueProxy, ValueMut as ValueMutProxy,
};
use crate::__private::registers::REGISTER_CURSOR;
use crate::__private::storage_traits::private::Sealed;
use crate::__private::{
    read_field, read_register, storage_cursor, storage_remove, storage_traits, write_field,
};
use crate::collections::hashmap::iterator::{HashMapIt, Keys, Values};
use nohash_hasher::IntMap;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;
use std::rc::Rc;
use xxhash_rust::xxh64::Xxh64;

// Enforces determinism when hashing keys
pub(crate) const SEED: u64 = 12345;

enum CacheOp {
    Update,
    Remove,
}

pub struct HashMap<K, V> {
    base_key: u64,
    cache: RefCell<IntMap<u64, Rc<RefCell<KeyValue<K, V>>>>>,
    operations: IntMap<u64, CacheOp>,
    entries: usize,
    loaded: Cell<bool>, // All entries in the DB have already been fetched
}

impl<K, V> Sealed for HashMap<K, V> {}

impl<K, V> storage_traits::ToPayload for HashMap<K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    fn to_payload(&self, path: &str) -> anyhow::Result<Option<String>> {
        // As this is a map, there is no further nesting
        if !path.is_empty() {
            return Ok(None);
        }
        // We build the json output manually to save performance
        let n_items = self.len();
        if n_items == 0 {
            return Ok(Some("{}".to_string()));
        }
        let mut items = self.iter();
        let first = items.next().unwrap(); // We checked empty

        // To pre-allocate the output string, we encode one object and use this as a reference
        let encoded = serde_json::to_string(first.as_ref())?;

        // for N items: N * ITEM_LENGTH + (N-1) (commas) + 2 ('{}'); add some padding just in case
        let mut buf = String::with_capacity(encoded.len() * n_items + n_items + 10);
        buf.push('{');
        buf.push_str(&encoded);
        buf.push(',');
        for item in items {
            let encoded = serde_json::to_string(item.as_ref())?;
            buf.push_str(&encoded);
            buf.push(',');
        }
        // Remove trailing ','
        if n_items > 1 {
            buf.pop();
        }
        buf.push('}');
        Ok(Some(buf))
    }
}

impl<K, V> Debug for HashMap<K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq + Debug,
    V: Serialize + DeserializeOwned + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Use the Rust's built-in debug_map helper
        let mut dm = f.debug_map();
        for entry in self.iter() {
            let (k, v) = &*entry;
            dm.entry(k, v);
        }
        dm.finish()
    }
}

impl<K, V> storage_traits::Storeable for HashMap<K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    fn decode(base_key: u64) -> Self {
        Self::open(base_key)
    }

    fn parse_value(value: Value, base_key: u64) -> anyhow::Result<Self> {
        let values: Vec<KeyValue<K, V>> = serde_json::from_value(value)?;
        let mut out = Self::new(base_key);
        for v in values {
            out.insert(v.pair.0, v.pair.1);
        }
        Ok(out)
    }

    fn commit(self, _base_key: u64) {
        self.commit()
    }
}

impl<K, V> HashMap<K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    pub(crate) fn new(base_key: u64) -> Self {
        HashMap {
            base_key,
            cache: RefCell::default(),
            operations: IntMap::default(),
            entries: 0,
            loaded: Cell::new(false),
        }
    }

    pub(crate) fn open(base_key: u64) -> Self {
        // Read number of entries from the DB
        let entries = read_field::<usize>(base_key, 0).unwrap();
        HashMap {
            base_key,
            cache: RefCell::default(),
            operations: IntMap::default(),
            entries,
            loaded: Cell::new(false),
        }
    }

    pub fn len(&self) -> usize {
        self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn contains_key(&self, sub_key: K) -> bool {
        let sub_key = Self::hash_key(&sub_key);
        self.read(sub_key).is_some()
    }

    pub fn remove(&mut self, key: K) -> Option<V> {
        let internal_key = Self::hash_key(&key);
        // Check if key is present
        self.read(internal_key)?;
        // Decrease number of entries
        self.entries = self.entries.saturating_sub(1);
        // Flag key as removed
        self.operations.insert(internal_key, CacheOp::Remove);
        // Remove value from cache
        let mut cache = self.cache.borrow_mut();
        cache.remove(&internal_key).and_then(Self::extract_cell)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let internal_key = Self::hash_key(&key);
        if self.read(internal_key).is_none() {
            // Increase number of entries
            self.entries = self.entries.saturating_add(1);
        }
        // Flag key as modified
        self.operations.insert(internal_key, CacheOp::Update);
        // Insert new value
        let mut cache = self.cache.borrow_mut();
        // Create pair
        let pair = KeyValue::new(key, value);
        let cell = Rc::new(RefCell::new(pair));
        // Insert new KeyPair into the cache
        cache
            .insert(internal_key, cell)
            .and_then(Self::extract_cell)
    }

    pub fn get(&self, key: K) -> Option<ValueProxy<'_, K, V>> {
        let internal_key = Self::hash_key(&key);
        match self.read(internal_key) {
            None => None,
            Some(cell) => {
                let proxy = ValueProxy {
                    cell_ptr: cell,
                    _back_ref: PhantomData,
                };
                Some(proxy)
            }
        }
    }

    pub fn get_mut(&mut self, key: K) -> Option<ValueMutProxy<'_, K, V>> {
        let internal_key = Self::hash_key(&key);
        match self.read(internal_key) {
            None => None,
            Some(cell) => {
                // NOTE: Mark the node as changed, because the user could totally do that.
                self.flag_write(internal_key);
                let proxy = ValueMutProxy {
                    cell_ptr: cell,
                    _back_ref: PhantomData,
                };
                Some(proxy)
            }
        }
    }

    pub fn clear(&mut self) {
        // Discard local changes
        self.operations.clear();
        // Load missing entries to memory
        self.load_entries();
        // Flag all keys to be removed
        for key in self.cache.borrow().keys() {
            self.operations.insert(*key, CacheOp::Remove);
        }
        // Update counter of total entries
        self.entries = 0;
        // Clear the in-memory map, deallocating the used resources
        self.cache = RefCell::default();
        // Flag loaded to false as we lost the DB entries
        self.loaded.set(false);
    }

    pub fn iter(&self) -> HashMapIt<K, V> {
        // Load missing entries to memory
        self.load_entries();
        HashMapIt::new(self)
    }

    pub fn keys(&self) -> Keys<K, V> {
        // Load missing entries to memory
        self.load_entries();
        // Return Keys iterator
        Keys::new(self)
    }

    pub fn values(&self) -> Values<K, V> {
        // Load missing entries to memory
        self.load_entries();
        // Return Values iterator
        Values::new(self)
    }

    fn commit(self) {
        // Store the entries counter in the DB
        write_field(self.base_key, 0, &self.entries);

        // Sync the in-memory mirror with the DB state
        for (key, op) in &self.operations {
            match op {
                CacheOp::Update => {
                    let cache = self.cache.borrow();
                    let cell = cache.get(key).expect("Cache corruption");
                    write_field(self.base_key, *key, cell.as_ref());
                }
                CacheOp::Remove => storage_remove(self.base_key, *key),
            }
        }
    }

    fn read(&self, key: u64) -> Option<Rc<RefCell<KeyValue<K, V>>>> {
        // Deleted keys (but still not commited) must return None
        // as the DB still contains them
        if let Some(CacheOp::Remove) = self.operations.get(&key) {
            return None;
        }
        // Check first the in-memory copy
        if let Some(cell) = self.cache.borrow().get(&key) {
            return Some(cell.clone());
        }
        // Fallback to DB
        match read_field::<KeyValue<K, V>>(self.base_key, key) {
            None => None,
            Some(keypair) => {
                let cell = Rc::new(RefCell::new(keypair));
                // Add value to the in-memory mirror
                self.cache.borrow_mut().insert(key, cell.clone());
                Some(cell)
            }
        }
    }

    fn load_entries(&self) {
        if self.loaded.get() {
            // The in-memory mirror already contains all the DB entries
            return;
        }
        // Flag loaded to true to avoid dumping the DB several times
        self.loaded.set(true);

        // Load missing entries to memory
        let entries = storage_cursor(self.base_key) as usize;

        for i in 0..entries {
            // Read content from register
            let bytes = read_register(REGISTER_CURSOR.saturating_add(i as u64))
                .expect("Fail to read register");
            // Convert into the sub-key
            let arr: [u8; 8] = bytes.as_slice().try_into().expect("Slice length error");
            let sub_key = u64::from_le_bytes(arr);
            // Load keys into the cache
            self.read(sub_key);
        }
    }

    fn flag_write(&mut self, key: u64) {
        self.operations.insert(key, CacheOp::Update);
    }

    fn get_entry(&self, index: usize) -> Option<EntryProxy<'_, K, V>> {
        // Read n-th key
        let key = *self.cache.borrow().keys().nth(index)?;
        // Create entry proxy object
        match self.read(key) {
            None => None,
            Some(cell) => {
                let key = EntryProxy {
                    cell_ptr: cell,
                    _back_ref: PhantomData,
                };
                Some(key)
            }
        }
    }

    fn get_key(&self, index: usize) -> Option<KeyProxy<'_, K, V>> {
        // Read n-th key
        let key = *self.cache.borrow().keys().nth(index)?;
        // Create key proxy object
        match self.read(key) {
            None => None,
            Some(cell) => {
                let key = KeyProxy {
                    cell_ptr: cell,
                    _back_ref: PhantomData,
                };
                Some(key)
            }
        }
    }

    fn get_value(&self, index: usize) -> Option<ValueProxy<'_, K, V>> {
        // Read n-th key
        let key = *self.cache.borrow().keys().nth(index)?;
        // Create value proxy object
        match self.read(key) {
            None => None,
            Some(cell) => {
                let key = ValueProxy {
                    cell_ptr: cell,
                    _back_ref: PhantomData,
                };
                Some(key)
            }
        }
    }

    fn extract_cell(rc: Rc<RefCell<KeyValue<K, V>>>) -> Option<V> {
        let old_cell = Rc::try_unwrap(rc).ok().expect("Rc strong counter > 1");
        Some(old_cell.into_inner().pair.1)
    }

    fn hash_key(key: &K) -> u64 {
        let mut h = Xxh64::new(SEED);
        key.hash(&mut h);
        h.digest()
    }
}
