use crate::__private::storage_traits::Storeable;
use crate::collections::lazyvec::LazyVec;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
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
        // Metadata base key is 0
        // LazyVec base keys are in range [1,16]
        let indices: [Key; SHARDS] = std::array::from_fn(|i| (i as Key) + 1);
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
        // LazyVec base keys are in range [1,16]
        let indices: [Key; SHARDS] = std::array::from_fn(|i| (i as Key) + 1);
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
}
