use borderless_kv_store::{Db, RawRead, RawWrite, Tx};
use borderless_sdk::{
    contract::{Description, Info, Introduction, Metadata},
    BorderlessId, ContractId,
    __private::storage_keys::*,
    hash::Hash256,
    http::ContractInfo,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::{action_log::ActionLog, logger::Logger, Result, CONTRACT_SUB_DB};

/// Model-controller to retrive information about a contract from the key-value storage.
pub struct Controller<'a, S: Db> {
    db: &'a S,
}

impl<'a, S: Db> Controller<'a, S> {
    pub fn new(db: &'a S) -> Self {
        Self { db }
    }

    pub fn actions(self, cid: ContractId) -> ActionLog<'a, S> {
        ActionLog::new(self.db, cid)
    }

    pub fn logs(self, cid: ContractId) -> Logger<'a, S> {
        Logger::new(self.db, cid)
    }

    pub fn contract_participants(&self, cid: &ContractId) -> Result<Option<Vec<BorderlessId>>> {
        self.read_value(cid, BASE_KEY_METADATA, META_SUB_KEY_PARTICIPANTS)
    }

    pub fn contract_exists(&self, cid: &ContractId) -> Result<bool> {
        Ok(self
            .read_value::<ContractId>(cid, BASE_KEY_METADATA, META_SUB_KEY_CONTRACT_ID)?
            .is_some())
    }

    pub fn contract_last_tx_hash(&self, cid: &ContractId) -> Result<Option<Hash256>> {
        let actions = ActionLog::new(self.db, *cid);
        match actions.last()? {
            Some(action) => Ok(Some(action.tx_ctx.tx_id.hash)),
            None => Ok(None),
        }
    }

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

    pub fn contract_desc(&self, cid: &ContractId) -> anyhow::Result<Option<Description>> {
        Ok(self.read_value(cid, BASE_KEY_METADATA, META_SUB_KEY_DESC)?)
    }

    pub fn contract_meta(&self, cid: &ContractId) -> anyhow::Result<Option<Metadata>> {
        Ok(self.read_value(cid, BASE_KEY_METADATA, META_SUB_KEY_META)?)
    }

    pub fn contract_full(&self, cid: &ContractId) -> anyhow::Result<Option<ContractInfo>> {
        let info = self.contract_info(cid)?;
        let desc = self.contract_desc(cid)?;
        let meta = self.contract_meta(cid)?;
        Ok(Some(ContractInfo { info, desc, meta }))
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

// Helper function to write fields with system-keys
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
    use borderless_sdk::__private::storage_keys::*;
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
