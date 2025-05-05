use crate::__private::storage_traits::Storeable;
use crate::collections::lazyvec::proxy::Proxy as LazyVecProxy;
use crate::collections::lazyvec::LazyVec;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::hash::Hash;
use std::marker::PhantomData;
use xxhash_rust::xxh64::Xxh64;

type Key = u64;

const SHARDS: usize = 16;
// Update the mask if SHARDS value is changed
const MASK: u64 = 0xF;
// Enforces determinism when hashing keys to index a shard
pub(crate) const SEED: u64 = 12345;

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
            // Shards are located in base_key + [1, 16]
            let offset = i.saturating_add(1) as u64;
            let shard_key = base_key.saturating_add(offset);
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
            // Shards are located in base_key + [1, 16]
            let offset = i.saturating_add(1) as u64;
            let shard_key = base_key.saturating_add(offset);
            LazyVec::decode(shard_key)
        });
        // Create Metadata object
        Metadata {
            shards,
            _ref: PhantomData,
        }
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
        let mut h = Xxh64::new(SEED);
        key.hash(&mut h);
        let hash = h.digest();
        // Extract the less-significant bits out of the hash
        (hash & MASK) as usize
    }

    pub(crate) fn insert(&mut self, key: K) {
        // Select the right shard
        let index = Self::shard_from_key(&key);
        // Push the new key
        self.shards[index].push(key);
    }

    pub(crate) fn remove(&mut self, key: K) {
        // Select the right shard
        let shard_idx = Self::shard_from_key(&key);
        // Remove the key if it exists
        if let Some(pos) = self.shards[shard_idx].iter().position(|k| *k == key) {
            self.shards[shard_idx].remove(pos);
        }
    }

    pub(crate) fn at(&self, idx: usize) -> Option<LazyVecProxy<'_, K>> {
        let mut pos = idx;
        for shard in &self.shards {
            if pos < shard.len() {
                return shard.get(pos);
            }
            pos = pos.saturating_sub(shard.len());
        }
        // Index was out of bounds
        None
    }
}
