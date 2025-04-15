use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use borderless_kv_store::{Db, RawRead, Tx};
use borderless_sdk::http::queries::Pagination;
use borderless_sdk::http::{PaginatedElements, TxAction};
use borderless_sdk::ContractId;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use borderless_sdk::contract::{CallAction, TxCtx};

use borderless_sdk::__private::storage_keys::{StorageKey, BASE_KEY_ACTION_LOG};

use crate::controller::{read_system_value, write_system_value};
use crate::{Result, CONTRACT_SUB_DB};

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
    db: &'a S,
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

    fn try_from(record: ActionRecord) -> std::result::Result<Self, Self::Error> {
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
        Self { db, cid }
    }

    pub(crate) fn commit(
        self,
        db_ptr: &S::Handle,
        txn: &mut <S as Db>::RwTx<'_>,
        action: &CallAction,
        tx_ctx: TxCtx,
    ) -> Result<()> {
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

    /// Retrieves a value from the log
    pub fn get(&self, idx: usize) -> Result<Option<ActionRecord>> {
        let idx = idx as u64;
        let len_commited = self.len()?;
        debug_assert!(idx < SUB_KEY_LOG_LEN);
        if idx < len_commited {
            self.read_value(BASE_KEY_ACTION_LOG, idx)
        } else {
            Ok(None)
        }
    }

    pub fn get_tx_action_paginated(
        &self,
        pagination: Pagination,
    ) -> Result<Option<PaginatedElements<TxAction>>> {
        // Get actions
        let n_actions = self.len()?;

        let mut elements = Vec::new();
        for idx in pagination.to_range() {
            // TODO: We can utilize the action log here !
            match self.read_value::<ActionRecord>(BASE_KEY_ACTION_LOG, idx as u64)? {
                Some(record) => {
                    let action = TxAction::try_from(record)?;
                    elements.push(action);
                }
                None => break,
            }
        }
        let paginated = PaginatedElements {
            elements,
            total_elements: n_actions as usize,
            pagination,
        };
        Ok(Some(paginated))
    }

    /// Retrieves the last action record
    pub fn last(&self) -> Result<Option<ActionRecord>> {
        let len_commited = self.len()?;
        self.read_value(BASE_KEY_ACTION_LOG, len_commited.saturating_sub(1))
    }

    pub fn len(&self) -> Result<u64> {
        Ok(self
            .read_value(BASE_KEY_ACTION_LOG, SUB_KEY_LOG_LEN)?
            .unwrap_or_default())
    }

    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }

    fn read_value<D: DeserializeOwned>(&self, base_key: u64, sub_key: u64) -> Result<Option<D>> {
        let db_ptr = self.db.open_sub_db(CONTRACT_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;
        let key = StorageKey::system_key(&self.cid, base_key, sub_key);
        let bytes = txn.read(&db_ptr, &key)?;
        let result = match bytes {
            Some(val) => Some(postcard::from_bytes(val)?),
            None => None,
        };
        txn.commit()?;
        Ok(result)
    }

    /// Returns an iterator over all action-records
    pub fn iter(&self) -> Iter<'_, S> {
        Iter { log: self, idx: 0 }
    }
}

/// Iterator over the [`ActionLog`]
pub struct Iter<'a, S: Db> {
    log: &'a ActionLog<'a, S>,
    idx: usize,
}

impl<'a, S: Db> Iterator for Iter<'a, S> {
    type Item = Result<ActionRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        self.idx += 1;
        self.log.get(idx).transpose()
    }
}
