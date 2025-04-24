use borderless::ContractId;
use borderless_kv_store::{Db, RawRead, RawWrite, Tx};
use lru::LruCache;
use parking_lot::Mutex;
use std::{num::NonZeroUsize, sync::Arc};
use vm::VmState;
use wasmtime::{Engine, Instance, Linker, Module, Store};

use crate::{Result, WASM_CODE_SUB_DB};

pub mod action_log;
pub mod contract;
pub mod logger;
pub mod swagent;
mod vm;

/// Storage for our webassembly code
#[derive(Clone)]
pub struct CodeStore<S: Db> {
    db: S,
    db_ptr: S::Handle,
    cache: Arc<Mutex<LruCache<ContractId, Instance, ahash::RandomState>>>,
}

impl<S: Db> CodeStore<S> {
    pub fn new(db: &S, cache_size: NonZeroUsize) -> Result<Self> {
        let db_ptr = db.create_sub_db(WASM_CODE_SUB_DB)?;
        let cache = LruCache::with_hasher(cache_size, ahash::RandomState::default());
        Ok(Self {
            db: db.clone(),
            db_ptr,
            cache: Arc::new(Mutex::new(cache)),
        })
    }

    pub fn insert_contract(&self, cid: ContractId, module: Module) -> Result<()> {
        let module_bytes = module.serialize()?;
        let mut txn = self.db.begin_rw_txn()?;
        txn.write(&self.db_ptr, &cid, &module_bytes)?;
        txn.commit()?;
        Ok(())
    }

    pub fn get_contract(
        &mut self,
        cid: &ContractId,
        engine: &Engine,
        store: &mut Store<VmState<S>>,
        linker: &mut Linker<VmState<S>>,
    ) -> Result<Option<Instance>> {
        if let Some(instance) = self.cache.lock().get(cid) {
            return Ok(Some(*instance));
        }
        let txn = self.db.begin_ro_txn()?;
        let module_bytes = txn.read(&self.db_ptr, cid)?;
        let module = match module_bytes {
            Some(bytes) => unsafe { Module::deserialize(engine, bytes)? },
            None => return Ok(None),
        };
        txn.commit()?;
        let instance = linker.instantiate(store, &module)?;
        self.cache.lock().push(*cid, instance);
        Ok(Some(instance))
    }

    pub fn available_contracts(&self) -> Result<Vec<ContractId>> {
        use borderless_kv_store::*;

        let mut out = Vec::new();
        let txn = self.db.begin_ro_txn()?;
        let mut cursor = txn.ro_cursor(&self.db_ptr)?;
        // TODO: if we store contracts and agents in the same db, we have to change the logic here
        for (key, _value) in cursor.iter() {
            let cid = ContractId::from_bytes(
                key.try_into()
                    .map_err(|_| crate::Error::msg("failed to parse contract-id from storage"))?,
            );
            out.push(cid);
        }
        drop(cursor);
        txn.commit()?;
        Ok(out)
    }
}
