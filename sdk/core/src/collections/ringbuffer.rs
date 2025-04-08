//! Ring-Buffer
//!
//! Ring-Buffer with fixed size

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::__private::read_field;

/// Storage key, where the meta-information about the buffer is saved
const SUB_KEY_META: u64 = u64::MAX;

pub struct RingBuffer<T> {
    base_key: u64,
    len_commited: u64,
    cache: Vec<Op<T>>,
}

#[derive(Serialize, Deserialize)]
struct BufferMeta {
    key_start: u64,
    key_end: u64,
    len_max: u64,
}

enum Op<T> {
    Push { idx: u64, value: T },
    Pop { idx: u64 },
}

impl<T> Op<T> {
    /// Helper function to count the number of items in the buffer from the cache
    ///
    /// Returns +1 for Op::Push and -1 for Op::Pop
    fn count(&self) -> i64 {
        match self {
            Op::Push { .. } => 1,
            Op::Pop { .. } => -1,
        }
    }
}

impl<T> RingBuffer<T> {
    pub fn new(base_key: u64, max_len: u64) -> Self {
        let meta = match read_field(base_key, SUB_KEY_META) {
            Some(val) => val,
            None => BufferMeta {
                key_start: 0,
                key_end: 0,
                len_max: max_len,
            },
        };
        Self {
            base_key,
            len_commited: meta.key_end - meta.key_start,
            cache: Vec::new(),
        }
    }
}

impl<T: Serialize + DeserializeOwned> RingBuffer<T> {
    /// Pushes a value to the buffer
    pub fn push(&mut self, value: T) {
        let idx = self.len();
        self.cache.push(Op::Push {
            idx: idx as u64,
            value,
        });
    }

    /// Remove an element from the buffer. If the buffer is already empty, this function returns `false`.
    pub fn pop(&mut self) -> bool {
        // If the cache is empty, we have to remove values from the database
        if self.cache.is_empty() {
            // Only pop, if there are commited elements
            if self.len_commited > 0 {
                self.cache.push(Op::Pop {
                    idx: self.len_commited - 1,
                });
                return true;
            } else {
                return false;
            }
        }
        // If the buffer is empty, we cannot pop any elements
        if self.is_empty() {
            return false;
        }

        // Now we have to check, if we actually can remove more values
        // let commited = self.len_commited as i64;
        // let cached = self.cache.iter().fold(0, |acc, val| acc + Op::count(val));
        todo!()
    }

    /// Returns the length of the buffer
    ///
    /// Values / Operations in the cache are taken into account.
    pub fn len(&self) -> u64 {
        let n_elements =
            self.len_commited as i64 + self.cache.iter().fold(0, |acc, val| acc + Op::count(val));
        assert!(n_elements >= 0, "cannot have less than 0 elements");
        n_elements as u64
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn commit(self) {
        todo!()
    }
}
