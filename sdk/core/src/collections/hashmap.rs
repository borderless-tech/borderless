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
use std::cell::{OnceCell, RefCell};
use std::collections::HashSet;
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
    db_keys: OnceCell<Vec<u64>>,
}

impl<K, V> Sealed for HashMap<K, V> {}

impl<K, V> storage_traits::ToPayload for HashMap<K, V>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
    V: Serialize + DeserializeOwned,
{
    fn to_payload(&self, path: &str) -> anyhow::Result<Option<String>> {
        use serde_json::to_string as to_str;
        // Remove leading '/' if present
        let path = path.strip_prefix('/').unwrap_or(path);

        let (path, remainder) = match path.split_once('/') {
            Some((path, remainder)) => (path, remainder),
            None => (path, ""),
        };

        if !path.is_empty() {
            // Try to deserialize key (complex key types will fail)
            let key: K = match serde_json::from_str(path) {
                Ok(key) => key,
                Err(_) => return Ok(None),
            };

            // Nest to the next to_payload()
            return self
                .get(key)
                .map_or(Ok(None), |val| (*val).to_payload(remainder));
        };
        // We build the json output manually to save performance
        let n_items = self.len();
        if n_items == 0 {
            return Ok(Some("{}".to_string()));
        }
        let first = self.iter().next().unwrap(); // We checked empty
        let complex_key = matches!(serde_json::to_value(first.key())?, Value::Object(_));

        let entries: Vec<String> = self
            .iter()
            .map(|item| {
                let k = to_str(item.key()).expect("Key serialization error");
                let v = to_str(item.value()).expect("Value serialization error");

                if complex_key {
                    format!("[{}, {}]", k, v)
                } else {
                    let k = to_str(&k).unwrap(); // Escape key
                    format!("{}: {}", k, v)
                }
            })
            .collect();

        let body = entries.join(",");

        let out = if complex_key {
            format!("[{}]", body)
        } else {
            format!("{{{}}}", body)
        };

        Ok(Some(out))
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
        let values: std::collections::HashMap<K, V> = serde_json::from_value(value)?;
        let mut out = Self::new(base_key);
        for v in values {
            out.insert(v.0, v.1);
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
            db_keys: OnceCell::default(),
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
            db_keys: OnceCell::default(),
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
        self.cache = RefCell::default();
        // Flag keys to be removed
        // TODO Enhance to avoid cloning the keys
        let db_keys: Vec<u64> = self.db_keys().to_vec();
        for key in db_keys {
            self.operations.insert(key, CacheOp::Remove);
        }
        // Update counter of total entries
        self.entries = 0;
    }

    pub fn iter(&self) -> HashMapIt<K, V> {
        HashMapIt::new(self)
    }

    pub fn keys(&self) -> Keys<K, V> {
        // Return Keys iterator
        Keys::new(self)
    }

    pub fn values(&self) -> Values<K, V> {
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

    fn flag_write(&mut self, key: u64) {
        self.operations.insert(key, CacheOp::Update);
    }

    fn get_entry(&self, internal_key: u64) -> Option<EntryProxy<'_, K, V>> {
        // Create entry proxy object
        match self.read(internal_key) {
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

    fn get_key(&self, internal_key: u64) -> Option<KeyProxy<'_, K, V>> {
        // Create key proxy object
        match self.read(internal_key) {
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

    fn get_value(&self, internal_key: u64) -> Option<ValueProxy<'_, K, V>> {
        // Create value proxy object
        match self.read(internal_key) {
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

    fn db_keys(&self) -> &Vec<u64> {
        self.db_keys.get_or_init(|| {
            // Load entries from DB
            let entries = storage_cursor(self.base_key) as usize;

            // Load DB keys
            let mut db_keys: Vec<u64> = Vec::with_capacity(entries);
            for i in 0..entries {
                // Read content from register
                let bytes = read_register(REGISTER_CURSOR.saturating_add(i as u64))
                    .expect("Fail to read register");
                // Convert into the sub-key
                let arr: [u8; 8] = bytes.as_slice().try_into().expect("Slice length error");
                let sub_key = u64::from_le_bytes(arr);
                // Push key into the vector
                db_keys.push(sub_key);
            }
            db_keys
        })
    }

    fn internal_keys(&self) -> HashSet<u64> {
        let cache = self.cache.borrow();
        cache.keys().chain(self.db_keys().iter()).cloned().collect()
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
