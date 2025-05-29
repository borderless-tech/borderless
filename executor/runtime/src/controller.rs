use borderless::{
    contracts::{Description, Info, Introduction, Metadata, Revocation, TxCtx},
    BorderlessId, ContractId,
    __private::storage_keys::*,
    hash::Hash256,
    http::ContractInfo,
};
use borderless_kv_store::{Db, RawRead, RawWrite, Tx};
use serde::{de::DeserializeOwned, Serialize};

use crate::{rt::action_log::ActionLog, rt::logger::Logger, Result, CONTRACT_SUB_DB};

// TODO: Add agent related functions as well

/// Model-controller to retrieve information about a contract from the key-value storage.
pub struct Controller<'a, S: Db> {
    db: &'a S,
}

impl<'a, S: Db> Controller<'a, S> {
    pub fn new(db: &'a S) -> Self {
        Self { db }
    }

    /// Returns the [`ActionLog`] of the contract
    pub fn actions(self, cid: ContractId) -> ActionLog<'a, S> {
        ActionLog::new(self.db, cid)
    }

    /// Returns the [`Logger`] of the contract
    pub fn logs(self, cid: ContractId) -> Logger<'a, S> {
        Logger::new(self.db, cid)
    }

    /// List of contract-participants
    pub fn contract_participants(&self, cid: &ContractId) -> Result<Option<Vec<BorderlessId>>> {
        self.read_value(cid, BASE_KEY_METADATA, META_SUB_KEY_PARTICIPANTS)
    }

    /// Returns `true` if the contract exists
    pub fn contract_exists(&self, cid: &ContractId) -> Result<bool> {
        Ok(self
            .read_value::<ContractId>(cid, BASE_KEY_METADATA, META_SUB_KEY_CONTRACT_ID)?
            .is_some())
    }

    /// Returns `true` if the contract has been revoked
    pub fn contract_revoked(&self, cid: &ContractId) -> Result<bool> {
        Ok(self.contract_revoked_ts(cid)?.is_some())
    }

    /// Returns the timestamp, when the contract has been revoked.
    pub fn contract_revoked_ts(&self, cid: &ContractId) -> Result<Option<u64>> {
        self.read_value::<u64>(cid, BASE_KEY_METADATA, META_SUB_KEY_REVOKED_TS)
    }

    /// Returns the hash of the last-tx that was executed by the contract
    pub fn contract_last_tx_hash(&self, cid: &ContractId) -> Result<Option<Hash256>> {
        let actions = ActionLog::new(self.db, *cid);
        match actions.last()? {
            Some(action) => Ok(Some(action.tx_ctx.tx_id.hash)),
            None => Ok(None),
        }
    }

    /// Returns the [`Info`] section of the contract
    pub fn contract_info(&self, cid: &ContractId) -> Result<Option<Info>> {
        let participants = self.read_value(cid, BASE_KEY_METADATA, META_SUB_KEY_PARTICIPANTS)?;
        let roles = self.read_value(cid, BASE_KEY_METADATA, META_SUB_KEY_ROLES)?;
        let sinks = self.read_value(cid, BASE_KEY_METADATA, META_SUB_KEY_SINKS)?;
        match (participants, roles, sinks) {
            (Some(participants), Some(roles), Some(sinks)) => Ok(Some(Info {
                contract_id: *cid,
                participants,
                roles,
                sinks,
            })),
            _ => Ok(None),
        }
    }

    /// Returns the [`Description`] of the contract
    pub fn contract_desc(&self, cid: &ContractId) -> Result<Option<Description>> {
        self.read_value(cid, BASE_KEY_METADATA, META_SUB_KEY_DESC)
    }

    /// Returns the [`Metadata`] of the contract
    pub fn contract_meta(&self, cid: &ContractId) -> Result<Option<Metadata>> {
        self.read_value(cid, BASE_KEY_METADATA, META_SUB_KEY_META)
    }

    /// Returns the full [`ContractInfo`], which bundles info, description and metadata.
    pub fn contract_full(&self, cid: &ContractId) -> Result<Option<ContractInfo>> {
        let info = self.contract_info(cid)?;
        let desc = self.contract_desc(cid)?;
        let meta = self.contract_meta(cid)?;
        Ok(Some(ContractInfo { info, desc, meta }))
    }

    /// Returns the [`Revocation`] of the contract, if any.
    pub fn contract_revocation(&self, cid: &ContractId) -> Result<Option<Revocation>> {
        self.read_value(cid, BASE_KEY_METADATA, META_SUB_KEY_REVOCATION)
    }

    fn read_value<D: DeserializeOwned>(
        &self,
        cid: &ContractId,
        base_key: u64,
        sub_key: u64,
    ) -> Result<Option<D>> {
        let db_ptr = self.db.open_sub_db(CONTRACT_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;
        let key = StorageKey::system_key(cid, base_key, sub_key);
        let bytes = txn.read(&db_ptr, &key)?;
        let result = match bytes {
            Some(val) => Some(postcard::from_bytes(val)?),
            None => None,
        };
        txn.commit()?;
        Ok(result)
    }
}

// Helper function to write fields with system-keys
pub(crate) fn write_system_value<S: Db, D: Serialize>(
    db_ptr: &S::Handle,
    txn: &mut <S as Db>::RwTx<'_>,
    cid: &ContractId,
    base_key: u64,
    sub_key: u64,
    data: &D,
) -> Result<()> {
    let key = StorageKey::system_key(cid, base_key, sub_key);
    let bytes = postcard::to_allocvec(data)?;
    txn.write(db_ptr, &key, &bytes)?;
    Ok(())
}

// Helper function to read fields from system-keys
pub(crate) fn read_system_value<S: Db, D: DeserializeOwned>(
    db_ptr: &S::Handle,
    txn: &<S as Db>::RwTx<'_>,
    cid: &ContractId,
    base_key: u64,
    sub_key: u64,
) -> Result<Option<D>> {
    let key = StorageKey::system_key(cid, base_key, sub_key);
    let bytes = txn.read(db_ptr, &key)?;
    match bytes {
        Some(val) => {
            let out = postcard::from_bytes(val)?;
            Ok(Some(out))
        }
        None => Ok(None),
    }
}

pub(crate) fn write_introduction<S: Db>(
    db_ptr: &S::Handle,
    txn: &mut <S as Db>::RwTx<'_>,
    introduction: &Introduction,
) -> Result<()> {
    use borderless::__private::storage_keys::*;
    let cid = introduction.contract_id;

    // Write contract-id
    write_system_value::<S, _>(
        db_ptr,
        txn,
        &cid,
        BASE_KEY_METADATA,
        META_SUB_KEY_CONTRACT_ID,
        &introduction.contract_id,
    )?;

    // Write participant list
    write_system_value::<S, _>(
        db_ptr,
        txn,
        &cid,
        BASE_KEY_METADATA,
        META_SUB_KEY_PARTICIPANTS,
        &introduction.participants,
    )?;

    // Write roles list
    write_system_value::<S, _>(
        db_ptr,
        txn,
        &cid,
        BASE_KEY_METADATA,
        META_SUB_KEY_ROLES,
        &introduction.roles,
    )?;

    // Write sink list
    write_system_value::<S, _>(
        db_ptr,
        txn,
        &cid,
        BASE_KEY_METADATA,
        META_SUB_KEY_SINKS,
        &introduction.sinks,
    )?;

    // Write description
    write_system_value::<S, _>(
        db_ptr,
        txn,
        &cid,
        BASE_KEY_METADATA,
        META_SUB_KEY_DESC,
        &introduction.desc,
    )?;

    // Write meta
    write_system_value::<S, _>(
        db_ptr,
        txn,
        &cid,
        BASE_KEY_METADATA,
        META_SUB_KEY_META,
        &introduction.meta,
    )?;

    // Write initial state
    write_system_value::<S, _>(
        db_ptr,
        txn,
        &cid,
        BASE_KEY_METADATA,
        META_SUB_KEY_INIT_STATE,
        &introduction.initial_state,
    )?;
    Ok(())
}

pub(crate) fn write_revocation<S: Db>(
    db_ptr: &S::Handle,
    txn: &mut <S as Db>::RwTx<'_>,
    revocation: &Revocation,
    tx_ctx: TxCtx,
    timestamp: u64,
) -> Result<()> {
    let cid = revocation.contract_id;
    // Update metadata field
    let meta: Option<Metadata> =
        read_system_value::<S, _>(db_ptr, txn, &cid, BASE_KEY_METADATA, META_SUB_KEY_META)?;
    let mut meta = meta.unwrap();

    meta.inactive_since = timestamp;
    meta.tx_ctx_revocation = Some(tx_ctx);

    write_system_value::<S, _>(
        db_ptr,
        txn,
        &cid,
        BASE_KEY_METADATA,
        META_SUB_KEY_META,
        &meta,
    )?;
    write_system_value::<S, _>(
        db_ptr,
        txn,
        &cid,
        BASE_KEY_METADATA,
        META_SUB_KEY_REVOKED_TS,
        &timestamp,
    )?;
    write_system_value::<S, _>(
        db_ptr,
        txn,
        &cid,
        BASE_KEY_METADATA,
        META_SUB_KEY_REVOCATION,
        &revocation,
    )?;
    Ok(())
}
