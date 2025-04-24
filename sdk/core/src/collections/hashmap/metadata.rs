use crate::__private::storage_traits::Storeable;
use crate::collections::lazyvec::LazyVec;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::marker::PhantomData;

type Key = u64;

const SHARDS: usize = 16;

#[derive(Serialize, Deserialize)]
pub(crate) struct Metadata<K> {
    base_key: Key,
    shards: [Key; SHARDS],
    len: usize,
    _ref: PhantomData<K>,
}

impl<K> Metadata<K>
where
    K: Serialize + DeserializeOwned,
{
    pub(crate) fn new(base_key: Key) -> Self {
        // LazyVec base keys are metadata base_key + [1,16]
        let indices: [Key; SHARDS] = std::array::from_fn(|i| (i as Key) + 1 + base_key);
        // Init and commit each LazyVec in memory
        for idx in indices {
            let vec = LazyVec::<K>::new(idx);
            vec.commit(idx);
        }
        // Create Metadata object
        Metadata {
            base_key,
            shards: indices,
            len: 0,
            _ref: PhantomData,
        }
    }

    pub(crate) fn open(base_key: Key) -> Self {
        let mut total_len: usize = 0;
        // LazyVec base keys are metadata base_key + [1,16]
        let indices: [Key; SHARDS] = std::array::from_fn(|i| (i as Key) + 1 + base_key);
        // Load each LazyVec into memory
        for idx in indices {
            let vec = LazyVec::<K>::decode(idx);
            total_len = total_len.saturating_add(vec.len());
            vec.commit(idx);
        }
        // Create Metadata object
        Metadata {
            base_key,
            shards: indices,
            len: total_len,
            _ref: PhantomData,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn keys(&self) -> Vec<Key> {
        todo!()
    }

    pub(crate) fn clear(&mut self) {
        todo!()
    }
}

impl<K> Metadata<K>
where
    K: Serialize + DeserializeOwned + Hash + Eq,
{
    fn shard_from_key(key: &K) -> usize {
        // Use the default Rust hasher, as it is very performant
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();
        // Extract the 4 less-significant bits out of the hash
        (hash & 0xF) as usize
    }

    pub(crate) fn insert(&self, key: K) {
        let index = Self::shard_from_key(&key);
        // Get the shard base key
        let shard_base_key = self.shards[index];
        // Load the corresponding LazyVec
        let mut vec: LazyVec<K> = LazyVec::decode(shard_base_key);
        // Push the new key
        vec.push(key)
    }

    pub(crate) fn remove(&self, key: K) {
        let index = Self::shard_from_key(&key);
        // Get the shard base key
        let shard_base_key = self.shards[index];
        // Load the corresponding LazyVec
        let _vec: LazyVec<K> = LazyVec::decode(shard_base_key);
        //vec.remove_elem();
        todo!()
    }
}
