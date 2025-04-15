use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use borderless_kv_store::Db;
use borderless_sdk::http::TxAction;
use borderless_sdk::ContractId;
use serde::{Deserialize, Serialize};

use borderless_sdk::contract::{CallAction, TxCtx};

use borderless_sdk::__private::storage_keys::BASE_KEY_ACTION_LOG;

use crate::vm::{read_system_value, write_system_value};

/// Sub-Key where the length of the action-log is stored
pub const SUB_KEY_LOG_LEN: u64 = u64::MAX;

// NOTE: This is the relationship that we want to save in the KV-Storage, when it comes to the actions
// - Action:
//   - Key: contract-id:ACTION_KEY:action-index
//   - Value: Action + Tx-Identifier
//   - Related to: Tx
// - Tx:
//   - Key: chain-id:block-number:block-tx-number
//   - Value: Tx-Info + Block-Id
//   - Related to: Block
// - Block:
//   - Key: chain-id:block-number
//   - Value: Block-Header + List of Txs
//   - Related to: Tx
// - Relate Tx-Hash -> chain-id:block-number:block-tx-number

/// The `ActionLog` records all actions that were successfully executed by the contract.
///
/// Since the action is fed into the contract as json-encoded bytes, we record exactly the raw json-bytes here,
/// and not the `CallAction` object. This allows us to efficiently give out the json object,
/// because instead of deserializing and then serializing it back to json, we can directly copy the json data after deserialization.
pub struct ActionLog<'a, S: Db> {
    _db: &'a S,
    cid: ContractId,
}

/// The `ActionRecord` is used to record actions in the [`ActionLog`].
///
/// The record bundles the raw json-bytes of the action together with meta-information like the transaction identifier
/// and transaction sequence number (which is the index of the transaction inside the block).
#[derive(Serialize, Deserialize)]
pub struct ActionRecord {
    pub tx_ctx: TxCtx,

    /// Action value as raw bytes.
    ///
    /// Since all incoming events are encoded in json, we directly save the json bytes here.
    /// This enables us to later directly spit out the json (for usage via api e.g.) without having to serialize it back.
    ///
    /// Note: This must decode to a [`CallAction`] object.
    #[serde(with = "serde_bytes")]
    pub value: Vec<u8>,

    /// Timestamp (as milliseconds since unix-epoch), when the action was commited.
    pub commited: u64,
}

impl TryFrom<ActionRecord> for TxAction {
    type Error = serde_json::Error;

    fn try_from(record: ActionRecord) -> Result<Self, Self::Error> {
        // Hm, I thought we could get around the additional parsing step here..
        // I still haven't given up ! TODO maybe construct the raw json value here, and see if this is faster.
        let action = serde_json::from_slice(&record.value)?;
        Ok(Self {
            tx_id: record.tx_ctx.tx_id,
            action,
            commited: record.commited,
        })
    }
}

impl<'a, S: Db> ActionLog<'a, S> {
    /// Opens (or creates) the action log
    pub fn new(db: &'a S, cid: ContractId) -> Self {
        Self { _db: db, cid }
    }

    // /// Returns `true` if the action log exists for the given contract.
    // ///
    // /// Basically checks, if the length of `0` has been written to the sub-key `SUB_KEY_LEN`.
    // pub fn exists() -> bool {
    //     storage_has_key(BASE_KEY_ACTION_LOG, SUB_KEY_LOG_LEN)
    // }

    // /// Pushes a new value to the log
    // pub fn push(&mut self, value_bytes: Vec<u8>, tx_ctx: TxCtx) {
    //     debug_assert!(self.len_commited < SUB_KEY_LOG_LEN);
    //     assert!(self.buffer.is_none(), "can only add one event at a time");
    //     self.buffer = Some(ActionRecord {
    //         tx_ctx,
    //         value: value_bytes,
    //         commited: 0,
    //     });
    // }

    // TODO: Add method for length

    pub(crate) fn commit(
        self,
        db_ptr: &S::Handle,
        txn: &mut <S as Db>::RwTx<'_>,
        action: &CallAction,
        tx_ctx: TxCtx,
    ) -> anyhow::Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("timestamp < 1970")?
            .as_millis()
            .try_into()
            .context("u64 should fit for 584942417 years")?;

        let len_commited: u64 = {
            read_system_value::<S, _>(db_ptr, txn, &self.cid, BASE_KEY_ACTION_LOG, SUB_KEY_LOG_LEN)?
                .unwrap_or_default()
        };

        let full_len = len_commited + 1;
        let sub_key = len_commited;
        let value = ActionRecord {
            tx_ctx,
            value: action.to_bytes()?,
            commited: timestamp,
        };
        write_system_value::<S, _>(db_ptr, txn, &self.cid, BASE_KEY_ACTION_LOG, sub_key, &value)?;
        write_system_value::<S, _>(
            db_ptr,
            txn,
            &self.cid,
            BASE_KEY_ACTION_LOG,
            SUB_KEY_LOG_LEN,
            &full_len,
        )?;

        log::debug!("Commited action to log. len={full_len}");
        Ok(())
    }

    // TODO: Remove this interface for production, this is super dangerous !
    // pub fn clear(&self) {
    //     let mut action_sub_key = 0;
    //     while storage_has_key(BASE_KEY_ACTION_LOG, action_sub_key) {
    //         storage_remove(BASE_KEY_ACTION_LOG, action_sub_key);
    //         action_sub_key += 1;
    //     }
    // }

    // /// Retrieves a value from the log
    // pub fn get(&self, idx: usize) -> Option<ActionRecord> {
    //     let idx = idx as u64;
    //     debug_assert!(idx < SUB_KEY_LOG_LEN);
    //     if idx < self.len_commited {
    //         read_field(BASE_KEY_ACTION_LOG, idx)
    //     } else {
    //         None
    //     }
    // }

    // /// Returns an iterator over all action-records
    // pub fn iter(&self) -> Iter<'_> {
    //     Iter { log: self, idx: 0 }
    // }
}

// /// Iterator over the [`ActionLog`]
// pub struct Iter<'a> {
//     log: &'a ActionLog,
//     idx: usize,
// }

// impl<'a> Iterator for Iter<'a> {
//     type Item = ActionRecord;

//     fn next(&mut self) -> Option<Self::Item> {
//         let idx = self.idx;
//         self.idx += 1;
//         self.log.get(idx)
//     }
// }
