// TODO: Is this smart ? Or should we just never do this from the wasm side of things ?
//
// Pro: We can use an acid transaction to commit the action together with the state
// Con: We have to create another copy of the action and have to encode it again,
//      which produces boilerplate
//
// A third option would be to make this guy just eat the plain Vec<u8> from the input,
// this way we avoid at least avoid the duplicate encoding.
//
use crate::{
    contract::CallAction,
    internal::{read_field, storage_write, write_field},
};

const SUB_KEY_LEN: u64 = u64::MAX;

// TODO: This is a very naive implementation, just to get going
pub struct ActionVec {
    base_key: u64,
    len_commited: u64,
    cache: Vec<CallAction>,
}

impl ActionVec {
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

impl ActionVec {
    /// Pushes a new value to the vector
    pub fn push(&mut self, value: CallAction) {
        debug_assert!(self.len_commited < SUB_KEY_LEN);
        self.cache.push(value);
    }

    /// Never call this directly ! This function is used by the macro !
    pub fn commit(self) {
        let full_len = self.len_commited + self.cache.len() as u64;
        for (idx, value) in self.cache.into_iter().enumerate() {
            let sub_key = self.len_commited + idx as u64;
            storage_write(self.base_key, sub_key, &value.to_bytes().unwrap())
        }
        write_field(self.base_key, SUB_KEY_LEN, &full_len);
    }
}

impl ActionVec {
    pub fn get(&self, idx: usize) -> Option<CallAction> {
        let idx = idx as u64;
        debug_assert!(idx < SUB_KEY_LEN);
        if idx < self.len_commited {
            read_field(self.base_key, idx)
        } else {
            let cache_idx = idx - self.len_commited;
            self.cache.get(cache_idx as usize).cloned()
        }
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter { vec: &self, idx: 0 }
    }
}

pub struct Iter<'a> {
    vec: &'a ActionVec,
    idx: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = CallAction;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        self.idx += 1;
        self.vec.get(idx)
    }
}
