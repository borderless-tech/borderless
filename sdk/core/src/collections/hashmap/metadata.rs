use crate::__private::storage_traits::Storeable;
use crate::collections::lazyvec::proxy::Proxy as LazyVecProxy;
use crate::collections::lazyvec::LazyVec;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::marker::PhantomData;

type Key = u64;

const SHARDS: usize = 16;
// Update the mask if SHARDS value is changed
const MASK: u64 = 0xF;

pub(crate) struct Metadata<K> {
    shards: [LazyVec<K>; SHARDS],
    _ref: PhantomData<K>,
}

impl<K> Metadata<K>
where
    K: Serialize + DeserializeOwned,
{
    pub(crate) fn new(base_key: Key) -> Self {
        // Init each shard
        let shards: [LazyVec<K>; SHARDS] = std::array::from_fn(|i| {
            let shard_key = base_key.saturating_add(i as Key);
            LazyVec::new(shard_key)
        });
        // Create Metadata object
        Metadata {
            shards,
            _ref: PhantomData,
        }
    }

    pub(crate) fn open(base_key: Key) -> Self {
        // Load each shard
        let shards: [LazyVec<K>; SHARDS] = std::array::from_fn(|i| {
            let shard_key = base_key.saturating_add(i as Key);
            LazyVec::decode(shard_key)
        });
        // Create Metadata object
        Metadata {
            shards,
            _ref: PhantomData,
        }
    }

    pub(crate) fn len_at_shard(&self, index: usize) -> usize {
        assert!(index < SHARDS);
        self.shards[index].len()
    }

    pub(crate) fn len(&self) -> usize {
        self.shards
            .iter()
            .map(|shard| shard.len())
            .fold(0, usize::saturating_add)
    }

    pub(crate) fn keys(&self) -> impl Iterator<Item = LazyVecProxy<'_, K>> + '_ {
        self.shards.iter().flat_map(|shard| shard.iter())
    }

    pub(crate) fn clear(&mut self) {
        self.shards.iter_mut().for_each(LazyVec::clear);
    }

    pub(crate) fn commit(self) {
        // Destructure to take ownership of the array
        let Metadata { shards, .. } = self;
        // Commit consumes each shard (the provided parameter is ignored)
        shards.into_iter().for_each(|shard| shard.commit(0))
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
        // Extract the less-significant bits out of the hash
        (hash & MASK) as usize
    }

    pub(crate) fn insert(&mut self, key: K) {
        // Select the right shard
        let index = Self::shard_from_key(&key);
        // Push the new key
        self.shards[index].push(key);
    }

    pub(crate) fn remove(&self, key: K) {
        // Select the right shard
        let _index = Self::shard_from_key(&key);
        // Remove the key
        //self.shards[index].remove_elem()
        todo!()
    }
}
