use serde::{Deserialize, Serialize};

use super::{
    read_field, storage_has_key, storage_keys::BASE_KEY_ACTION_LOG, storage_remove, write_field,
};

/// Sub-Key where the length of the action-log is stored
const SUB_KEY_LEN: u64 = u64::MAX;

/// The `ActionLog` records all actions that were successfully executed by the contract.
///
/// Since the action is fed into the contract as json-encoded bytes, we record exactly the raw json-bytes here,
/// and not the `CallAction` object. This allows us to efficiently give out the json object,
/// because instead of deserializing and then serializing it back to json, we can directly copy the json data after deserialization.
pub struct ActionLog {
    len_commited: u64,
    buffer: Option<ActionRecord>,
}

/// The `ActionRecord` is used to record actions in the [`ActionLog`].
///
/// The record bundles the raw json-bytes of the action together with meta-information like the transaction identifier
/// and transaction sequence number (which is the index of the transaction inside the block).
#[derive(Serialize, Deserialize)]
pub struct ActionRecord {
    // tx_id: TxId,
    pub tx_sq_number: u64,

    /// Action value as raw bytes.
    ///
    /// Since all incoming events are encoded in json, we directly save the json bytes here.
    /// This enables us to later directly spit out the json (for usage via api e.g.) without having to serialize it back.
    ///
    /// Note: This must decode to a [`CallAction`] object.
    #[serde(with = "serde_bytes")]
    pub value: Vec<u8>,
}

impl ActionLog {
    /// Opens (or creates) the action log
    pub fn open() -> Self {
        let len: u64 = if let Some(len) = read_field(BASE_KEY_ACTION_LOG, SUB_KEY_LEN) {
            len
        } else {
            write_field(BASE_KEY_ACTION_LOG, SUB_KEY_LEN, &0u64);
            0
        };
        Self {
            len_commited: len,
            buffer: None,
        }
    }

    /// Returns `true` if the action log exists for the given contract.
    ///
    /// Basically checks, if the length of `0` has been written to the sub-key `SUB_KEY_LEN`.
    pub fn exists() -> bool {
        storage_has_key(BASE_KEY_ACTION_LOG, SUB_KEY_LEN)
    }

    /// Pushes a new value to the log
    pub fn push(&mut self, value_bytes: Vec<u8>, tx_sq_number: u64) {
        debug_assert!(self.len_commited < SUB_KEY_LEN);
        assert!(self.buffer.is_none(), "can only add one event at a time");
        self.buffer = Some(ActionRecord {
            tx_sq_number,
            value: value_bytes,
        });
    }

    /// Never call this directly ! This function is used by the macro !
    pub fn commit(self) {
        assert!(
            self.buffer.is_some(),
            "commit must only be called, if there was an event"
        );
        let full_len = self.len_commited + 1;
        let sub_key = self.len_commited;
        write_field(BASE_KEY_ACTION_LOG, sub_key, &self.buffer.unwrap());
        write_field(BASE_KEY_ACTION_LOG, SUB_KEY_LEN, &full_len);
    }

    // TODO: Remove this interface for production, this is super dangerous !
    pub fn clear(&self) {
        let mut action_sub_key = 0;
        while storage_has_key(BASE_KEY_ACTION_LOG, action_sub_key) {
            storage_remove(BASE_KEY_ACTION_LOG, action_sub_key);
            action_sub_key += 1;
        }
    }

    /// Retrieves a value from the log
    pub fn get(&self, idx: usize) -> Option<ActionRecord> {
        let idx = idx as u64;
        debug_assert!(idx < SUB_KEY_LEN);
        if idx < self.len_commited {
            read_field(BASE_KEY_ACTION_LOG, idx)
        } else {
            None
        }
    }

    /// Returns an iterator over all action-records
    pub fn iter(&self) -> Iter<'_> {
        Iter { log: &self, idx: 0 }
    }
}

/// Iterator over the [`ActionLog`]
pub struct Iter<'a> {
    log: &'a ActionLog,
    idx: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = ActionRecord;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        self.idx += 1;
        self.log.get(idx)
    }
}
