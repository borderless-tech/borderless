use serde::{de::DeserializeOwned, Serialize};

use crate::internal::{read_field, write_field};

const SUB_KEY_LEN: u64 = u64::MAX;

// TODO: This is a very naive implementation, just to get going
pub struct AppendVec<T> {
    base_key: u64,
    len_commited: u64,
    cache: Vec<T>,
}

impl<T> AppendVec<T> {
    pub fn new(base_key: u64) -> Self {
        let len: u64 = if let Some(len) = read_field(base_key, SUB_KEY_LEN) {
            len
        } else {
            write_field(base_key, SUB_KEY_LEN, &0u64);
            0
        };
        Self {
            base_key,
            len_commited: len,
            cache: Vec::new(),
        }
    }

    pub fn len(&self) -> u64 {
        self.len_commited + self.cache.len() as u64
    }
}

impl<T: Serialize> AppendVec<T> {
    /// Pushes a new value to the vector
    pub fn push(&mut self, value: T) {
        debug_assert!(self.len_commited < SUB_KEY_LEN);
        self.cache.push(value);
    }

    /// Never call this directly ! This function is used by the macro !
    pub fn commit(self) {
        for (idx, value) in self.cache.into_iter().enumerate() {
            let sub_key = self.len_commited + 1 + idx as u64;
            write_field(self.base_key, sub_key, &value);
        }
    }
}

impl<T: DeserializeOwned + Clone> AppendVec<T> {
    pub fn get(&self, idx: usize) -> Option<T> {
        let idx = idx as u64;
        debug_assert!(idx < SUB_KEY_LEN);
        if idx < self.len_commited {
            read_field(self.base_key, idx)
        } else {
            let cache_idx = idx - self.len_commited;
            self.cache.get(cache_idx as usize).cloned()
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter { vec: &self, idx: 0 }
    }
}

pub struct Iter<'a, T> {
    vec: &'a AppendVec<T>,
    idx: usize,
}

impl<'a, T: DeserializeOwned + Clone> Iterator for Iter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        self.idx += 1;
        self.vec.get(idx)
    }
}
