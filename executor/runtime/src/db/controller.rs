use super::{
    action_log::{ActionLog, ActionRecord, RelTxAction},
    ledger::Ledger,
    logger::Logger,
    subscriptions::SubscriptionHandler,
};
use crate::{Result, ACTION_TX_REL_SUB_DB, AGENT_SUB_DB, CONTRACT_SUB_DB, SUBSCRIPTION_REL_SUB_DB};
use borderless::common::Participant;
use borderless::{
    common::{Description, Metadata, Revocation},
    contracts::Info,
    ContractId,
    __private::storage_keys::*,
    events::Sink,
    hash::Hash256,
    http::{AgentInfo, ContractInfo},
    pkg::{Source, SourceFlattened, WasmPkg, WasmPkgNoSource},
    prelude::{Id, TxCtx},
    AgentId, TxIdentifier,
};
use borderless_kv_store::*;
use serde::de::DeserializeOwned;

/// Model-controller to retrieve information about a contract from the key-value storage.
pub struct Controller<'a, S: Db> {
    db: &'a S,
}

impl<'a, S: Db> Controller<'a, S> {
    pub fn new(db: &'a S) -> Self {
        Self { db }
    }

    /// Returns the [`ActionLog`] of the contract
    pub fn actions(&self, cid: ContractId) -> ActionLog<'a, S> {
        ActionLog::new(self.db, cid)
    }

    /// Returns the [`Logger`] of the contract or agent
    pub fn logs(&self, id: impl Into<Id>) -> Logger<'a, S> {
        Logger::new(self.db, id)
    }

    /// Returns the [`Ledger`]
    pub fn ledger(&self) -> Ledger<'a, S> {
        Ledger::new(self.db)
    }

    /// List of contract-participants
    pub fn contract_participants(&self, cid: &ContractId) -> Result<Option<Vec<Participant>>> {
        self.read_value(
            &Id::contract(*cid),
            BASE_KEY_METADATA,
            META_SUB_KEY_PARTICIPANTS,
        )
    }

    /// Returns `true` if the contract exists
    pub fn contract_exists(&self, cid: &ContractId) -> Result<bool> {
        Ok(self
            .read_value::<ContractId>(&Id::contract(*cid), BASE_KEY_METADATA, META_SUB_KEY_ID)?
            .is_some())
    }

    /// Returns `true` if the contract exists
    pub fn agent_exists(&self, aid: &AgentId) -> Result<bool> {
        Ok(self
            .read_value::<AgentId>(&Id::agent(*aid), BASE_KEY_METADATA, META_SUB_KEY_ID)?
            .is_some())
    }

    /// Returns `true` if the contract has been revoked
    pub fn contract_revoked(&self, cid: &ContractId) -> Result<bool> {
        Ok(self.contract_revoked_ts(cid)?.is_some())
    }

    /// Returns `true` if the agent has been revoked
    pub fn agent_revoked(&self, aid: &AgentId) -> Result<bool> {
        Ok(self.agent_revoked_ts(aid)?.is_some())
    }

    /// Returns the timestamp, when the contract has been revoked.
    pub fn contract_revoked_ts(&self, cid: &ContractId) -> Result<Option<u64>> {
        self.read_value::<u64>(
            &Id::contract(*cid),
            BASE_KEY_METADATA,
            META_SUB_KEY_REVOKED_TS,
        )
    }

    /// Returns the timestamp, when the agent has been revoked.
    pub fn agent_revoked_ts(&self, aid: &AgentId) -> Result<Option<u64>> {
        self.read_value::<u64>(&Id::agent(*aid), BASE_KEY_METADATA, META_SUB_KEY_REVOKED_TS)
    }

    /// Returns the hash of the last-tx that was executed by the contract
    pub fn contract_last_tx_hash(&self, cid: &ContractId) -> Result<Option<Hash256>> {
        let actions = ActionLog::new(self.db, *cid);
        if let Some(action) = actions.last()? {
            return Ok(Some(action.tx_ctx.tx_id.hash));
        }
        // If there are no actions, the contract could simply be just introduced.
        // In this case, read the introduction and parse the tx_introduced value
        match self.contract_meta(cid)? {
            Some(meta) => Ok(meta.tx_ctx_introduction.map(|t| t.tx_id.hash)),
            None => Ok(None),
        }
    }

    /// Returns the [`Info`] section of the contract
    pub fn contract_info(&self, cid: &ContractId) -> Result<Option<Info>> {
        let id = Id::contract(*cid);
        let participants = self.read_value(&id, BASE_KEY_METADATA, META_SUB_KEY_PARTICIPANTS)?;
        let sinks = self.read_value(&id, BASE_KEY_METADATA, META_SUB_KEY_SINKS)?;
        match (participants, sinks) {
            (Some(participants), Some(sinks)) => Ok(Some(Info {
                contract_id: *cid,
                participants,
                sinks,
            })),
            _ => Ok(None),
        }
    }

    /// Returns the sinks for an agent
    ///
    /// Since an agent has no participants of roles, the sinks and the agent-id are the only two things
    /// that are left from the [`Info`] struct.
    pub fn agent_sinks(&self, aid: &AgentId) -> Result<Option<Vec<Sink>>> {
        let aid = Id::agent(*aid);
        self.read_value(&aid, BASE_KEY_METADATA, META_SUB_KEY_SINKS)
    }

    pub fn agent_subs(&self, aid: &AgentId) -> Result<Vec<String>> {
        let db_ptr = self.db.open_sub_db(SUBSCRIPTION_REL_SUB_DB)?;
        let mut txn = self.db.begin_ro_txn()?;
        SubscriptionHandler::<S>::get_subscriptions(&db_ptr, &mut txn, *aid)
    }

    /// Returns the [`Description`] of the contract
    pub fn contract_desc(&self, cid: &ContractId) -> Result<Option<Description>> {
        self.read_value(&Id::contract(*cid), BASE_KEY_METADATA, META_SUB_KEY_DESC)
    }

    /// Returns the [`Description`] of the agent
    pub fn agent_desc(&self, aid: &AgentId) -> Result<Option<Description>> {
        self.read_value(&Id::agent(*aid), BASE_KEY_METADATA, META_SUB_KEY_DESC)
    }

    /// Returns the [`Metadata`] of the contract
    pub fn contract_meta(&self, cid: &ContractId) -> Result<Option<Metadata>> {
        self.read_value(&Id::contract(*cid), BASE_KEY_METADATA, META_SUB_KEY_META)
    }

    /// Returns the [`Metadata`] of the agent
    pub fn agent_meta(&self, aid: &AgentId) -> Result<Option<Metadata>> {
        self.read_value(&Id::agent(*aid), BASE_KEY_METADATA, META_SUB_KEY_META)
    }

    /// Returns the full [`ContractInfo`], which bundles info, description and metadata.
    pub fn contract_full(&self, cid: &ContractId) -> Result<Option<ContractInfo>> {
        let info = self.contract_info(cid)?;
        let desc = self.contract_desc(cid)?;
        let meta = self.contract_meta(cid)?;
        Ok(Some(ContractInfo { info, desc, meta }))
    }

    /// Returns the full [`AgentInfo`], which bundles info, description and metadata.
    pub fn agent_full(&self, aid: &AgentId) -> Result<Option<AgentInfo>> {
        let sinks = self.agent_sinks(aid)?.unwrap_or_default();
        let subs = self.agent_subs(aid)?;
        let desc = self.agent_desc(aid)?;
        let meta = self.agent_meta(aid)?;
        Ok(Some(AgentInfo {
            agent_id: *aid,
            sinks,
            subs,
            desc,
            meta,
        }))
    }

    /// Returns the [`Revocation`] of the contract, if any.
    pub fn contract_revocation(&self, cid: &ContractId) -> Result<Option<Revocation>> {
        self.read_value(
            &Id::contract(*cid),
            BASE_KEY_METADATA,
            META_SUB_KEY_REVOCATION,
        )
    }

    /// Returns the [`Revocation`] of the contract, if any.
    pub fn agent_revocation(&self, aid: &AgentId) -> Result<Option<Revocation>> {
        self.read_value(&Id::agent(*aid), BASE_KEY_METADATA, META_SUB_KEY_REVOCATION)
    }

    /// Queries an [`ActionRecord`] based on the [`TxIdentifier`]
    pub fn query_action(&self, tx_id: &TxIdentifier) -> Result<Option<ActionRecord>> {
        let tx_id_bytes = tx_id.to_bytes();
        let relation = {
            let rel_db = self.db.create_sub_db(ACTION_TX_REL_SUB_DB)?;
            let txn = self.db.begin_ro_txn()?;
            match txn.read(&rel_db, &tx_id_bytes)? {
                Some(bytes) => RelTxAction::from_bytes(&bytes),
                None => return Ok(None),
            }
        };
        // Do a sanity-check before we return the record
        match self
            .actions(relation.cid)
            .get(relation.action_idx as usize)?
        {
            Some(record) => {
                debug_assert!(record.tx_ctx.tx_id == *tx_id, "tx-id must match");
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    /// Returns the package definition for an agent
    pub fn agent_pkg_def(&self, aid: &AgentId) -> Result<Option<WasmPkgNoSource>> {
        self.read_value(
            &Id::agent(*aid),
            BASE_KEY_METADATA,
            META_SUB_KEY_PACKAGE_DEF,
        )
    }

    /// Returns the package definition for an agent
    pub fn agent_pkg_source(&self, aid: &AgentId) -> Result<Option<Source>> {
        let source: Option<SourceFlattened> = self.read_value(
            &Id::agent(*aid),
            BASE_KEY_METADATA,
            META_SUB_KEY_PACKAGE_SOURCE,
        )?;
        Ok(source.map(|s| s.unflatten()))
    }

    /// Returns the package definition for an agent
    pub fn agent_pkg_full(&self, aid: &AgentId) -> Result<Option<WasmPkg>> {
        let pkg_def = self.agent_pkg_def(aid)?;
        let source = self.agent_pkg_source(aid)?;
        match (pkg_def, source) {
            (Some(pkg), Some(source)) => Ok(Some(WasmPkg::from_def_and_source(pkg, source))),
            _ => Ok(None),
        }
    }

    /// Returns the package definition for an contract
    pub fn contract_pkg_def(&self, aid: &ContractId) -> Result<Option<WasmPkgNoSource>> {
        self.read_value(
            &Id::contract(*aid),
            BASE_KEY_METADATA,
            META_SUB_KEY_PACKAGE_DEF,
        )
    }

    /// Returns the package definition for an contract
    pub fn contract_pkg_source(&self, aid: &ContractId) -> Result<Option<Source>> {
        // NOTE: We write a flattened source to the disk, because postcard does not support all serde features
        let source: Option<SourceFlattened> = self.read_value(
            &Id::contract(*aid),
            BASE_KEY_METADATA,
            META_SUB_KEY_PACKAGE_SOURCE,
        )?;
        Ok(source.map(|s| s.unflatten()))
    }

    /// Returns the package definition for an contract
    pub fn contract_pkg_full(&self, aid: &ContractId) -> Result<Option<WasmPkg>> {
        let pkg_def = self.contract_pkg_def(aid)?;
        let source = self.contract_pkg_source(aid)?;
        match (pkg_def, source) {
            (Some(pkg), Some(source)) => Ok(Some(WasmPkg::from_def_and_source(pkg, source))),
            _ => Ok(None),
        }
    }

    fn read_value<D: DeserializeOwned>(
        &self,
        id: &Id,
        base_key: u64,
        sub_key: u64,
    ) -> Result<Option<D>> {
        // Use correct sub-db based on the id-type
        let db_ptr = match id {
            Id::Contract { .. } => self.db.open_sub_db(CONTRACT_SUB_DB)?,
            Id::Agent { .. } => self.db.open_sub_db(AGENT_SUB_DB)?,
        };
        let txn = self.db.begin_ro_txn()?;
        let key = StorageKey::system_key(id, base_key, sub_key);
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
#[cfg(any(feature = "contracts", feature = "agents"))]
pub(crate) fn write_system_value<S: Db, D: serde::Serialize, ID: AsRef<[u8; 16]>>(
    db_ptr: &S::Handle,
    txn: &mut <S as Db>::RwTx<'_>,
    uid: ID,
    base_key: u64,
    sub_key: u64,
    data: &D,
) -> Result<()> {
    let key = StorageKey::system_key(uid, base_key, sub_key);
    let bytes = postcard::to_allocvec(data)?;
    txn.write(db_ptr, &key, &bytes)?;
    Ok(())
}

// Helper function to write fields with system-keys
#[cfg(any(feature = "contracts", feature = "agents"))]
pub(crate) fn read_system_value<S: Db, D: DeserializeOwned, ID: AsRef<[u8; 16]>>(
    db_ptr: &S::Handle,
    txn: &<S as Db>::RwTx<'_>,
    cid: ID,
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

#[cfg(any(feature = "contracts", feature = "agents"))]
pub(crate) fn write_introduction<S: Db>(
    db_ptr: &S::Handle,
    subs_ptr: &S::Handle,
    txn: &mut <S as Db>::RwTx<'_>,
    introduction: borderless::common::Introduction,
) -> Result<()> {
    use borderless::__private::storage_keys::*;

    use crate::error::ErrorKind;
    let id = introduction.id;

    // NOTE: If the id was already written to disk, it means
    // that the contract/sw-agent has already been written !
    let check_id =
        read_system_value::<S, Id, _>(db_ptr, txn, &id, BASE_KEY_METADATA, META_SUB_KEY_ID)?;
    if check_id.is_some() {
        return Err(ErrorKind::DoubleIntroduction.into());
    }

    // Write contract or sw-agent id
    write_system_value::<S, _, _>(
        db_ptr,
        txn,
        &id,
        BASE_KEY_METADATA,
        META_SUB_KEY_ID,
        &introduction.id,
    )?;

    // Write participant list
    write_system_value::<S, _, _>(
        db_ptr,
        txn,
        &id,
        BASE_KEY_METADATA,
        META_SUB_KEY_PARTICIPANTS,
        &introduction.participants,
    )?;

    // Write sink list
    write_system_value::<S, _, _>(
        db_ptr,
        txn,
        &id,
        BASE_KEY_METADATA,
        META_SUB_KEY_SINKS,
        &introduction.sinks,
    )?;

    // Write description
    write_system_value::<S, _, _>(
        db_ptr,
        txn,
        &id,
        BASE_KEY_METADATA,
        META_SUB_KEY_DESC,
        &introduction.desc,
    )?;

    // Write meta
    write_system_value::<S, _, _>(
        db_ptr,
        txn,
        &id,
        BASE_KEY_METADATA,
        META_SUB_KEY_META,
        &introduction.meta,
    )?;

    // Write initial state
    write_system_value::<S, _, _>(
        db_ptr,
        txn,
        &id,
        BASE_KEY_METADATA,
        META_SUB_KEY_INIT_STATE,
        &introduction.initial_state,
    )?;

    // Write subscriptions
    match id {
        Id::Contract { .. } => {} // Not applicable
        Id::Agent { agent_id } => {
            for s in introduction.subscriptions {
                SubscriptionHandler::<S>::subscribe(subs_ptr, txn, agent_id, s)?
            }
        }
    }

    // Write package and source (flattened, because postcard does not support untagged enums)
    let (pkg_def, pkg_source) = introduction.package.into_def_and_source();
    let pkg_source = pkg_source.flatten();

    // Write pkg-def
    write_system_value::<S, _, _>(
        db_ptr,
        txn,
        &id,
        BASE_KEY_METADATA,
        META_SUB_KEY_PACKAGE_DEF,
        &pkg_def,
    )?;

    // Write pkg-source
    write_system_value::<S, _, _>(
        db_ptr,
        txn,
        &id,
        BASE_KEY_METADATA,
        META_SUB_KEY_PACKAGE_SOURCE,
        &pkg_source,
    )?;

    Ok(())
}

#[cfg(any(feature = "contracts", feature = "agents"))]
pub(crate) fn write_revocation<S: Db>(
    db_ptr: &S::Handle,
    txn: &mut <S as Db>::RwTx<'_>,
    revocation: &Revocation,
    timestamp: u64,
    tx_ctx: Option<TxCtx>,
) -> Result<()> {
    let cid = revocation.id;
    // Update metadata field
    let meta: Option<Metadata> =
        read_system_value::<S, _, _>(db_ptr, txn, &cid, BASE_KEY_METADATA, META_SUB_KEY_META)?;
    let mut meta = meta.unwrap();

    meta.inactive_since = timestamp;
    meta.tx_ctx_revocation = tx_ctx;

    write_system_value::<S, _, _>(
        db_ptr,
        txn,
        &cid,
        BASE_KEY_METADATA,
        META_SUB_KEY_META,
        &meta,
    )?;
    write_system_value::<S, _, _>(
        db_ptr,
        txn,
        &cid,
        BASE_KEY_METADATA,
        META_SUB_KEY_REVOKED_TS,
        &timestamp,
    )?;
    write_system_value::<S, _, _>(
        db_ptr,
        txn,
        &cid,
        BASE_KEY_METADATA,
        META_SUB_KEY_REVOCATION,
        &revocation,
    )?;
    Ok(())
}
