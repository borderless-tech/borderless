#[cfg(feature = "contracts")]
pub mod contract;

#[cfg(feature = "agents")]
pub mod swagent;

#[cfg(any(feature = "contracts", feature = "agents"))]
mod vm;

#[cfg(any(feature = "contracts", feature = "agents"))]
pub mod code_store {
    use super::vm::VmState;
    use borderless::{aid_prefix, cid_prefix, AgentId, ContractId};
    use borderless_kv_store::{Db, RawRead, RawWrite, Tx};
    use lru::LruCache;
    use parking_lot::Mutex;
    use std::{num::NonZeroUsize, sync::Arc};
    use wasmtime::{Engine, Instance, Linker, Module, Store};

    use crate::{Result, WASM_CODE_SUB_DB};

    /// Generalized ID - this is either a Contract-ID or an Agent-ID
    type Id = [u8; 16];

    /// Storage for our webassembly code
    #[derive(Clone)]
    pub struct CodeStore<S: Db> {
        db: S,
        db_ptr: S::Handle,
        cache: Arc<Mutex<LruCache<Id, Instance, ahash::RandomState>>>,
    }

    impl<S: Db> CodeStore<S> {
        pub fn new(db: &S) -> Result<Self> {
            Self::with_cache_size(db, NonZeroUsize::new(16).unwrap())
        }

        pub fn with_cache_size(db: &S, cache_size: NonZeroUsize) -> Result<Self> {
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

        pub fn insert_swagent(&self, aid: AgentId, module: Module) -> Result<()> {
            let module_bytes = module.serialize()?;
            let mut txn = self.db.begin_rw_txn()?;
            txn.write(&self.db_ptr, &aid, &module_bytes)?;
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
            if let Some(instance) = self.cache.lock().get(cid.as_bytes()) {
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
            self.cache.lock().push(*cid.as_bytes(), instance);
            Ok(Some(instance))
        }

        pub async fn get_agent(
            &mut self,
            aid: &AgentId,
            engine: &Engine,
            store: &mut Store<VmState<S>>,
            linker: &mut Linker<VmState<S>>,
        ) -> Result<Option<Instance>> {
            if let Some(instance) = self.cache.lock().get(aid.as_bytes()) {
                return Ok(Some(*instance));
            }
            let module = match self.read_module(aid, engine)? {
                Some(m) => m,
                None => return Ok(None),
            };
            let instance = linker.instantiate_async(store, &module).await?;
            self.cache.lock().push(*aid.as_bytes(), instance);
            Ok(Some(instance))
        }

        /// Helper function to read a module from the kv-storage
        ///
        /// Note: This helper function is required, because otherwise the compiler might complain
        /// that `RoTx` does not implement `Send`, as it cannot figure out on its own,
        /// that the transaction is dropped before the next `.await` point.
        fn read_module(
            &mut self,
            key: impl AsRef<[u8]>,
            engine: &Engine,
        ) -> Result<Option<Module>> {
            let txn = self.db.begin_ro_txn()?;
            let module_bytes = txn.read(&self.db_ptr, &key)?;
            let module = match module_bytes {
                Some(bytes) => unsafe { Module::deserialize(engine, bytes)? },
                None => return Ok(None),
            };
            txn.commit()?;
            Ok(Some(module))
        }

        pub fn available_contracts(&self) -> Result<Vec<ContractId>> {
            use borderless_kv_store::*;

            let mut out = Vec::new();
            let txn = self.db.begin_ro_txn()?;
            let mut cursor = txn.ro_cursor(&self.db_ptr)?;
            // NOTE: We have to filter out all keys without the cid prefix
            for (key, _value) in cursor.iter().filter(|(key, _)| cid_prefix(key)) {
                let cid =
                    ContractId::from_bytes(key.try_into().map_err(|_| {
                        crate::Error::msg("failed to parse contract-id from storage")
                    })?);
                out.push(cid);
            }
            drop(cursor);
            txn.commit()?;
            Ok(out)
        }

        pub fn available_swagents(&self) -> Result<Vec<AgentId>> {
            use borderless_kv_store::*;

            let mut out = Vec::new();
            let txn = self.db.begin_ro_txn()?;
            let mut cursor = txn.ro_cursor(&self.db_ptr)?;
            // NOTE: We have to filter out all keys without the aid prefix
            for (key, _value) in cursor.iter().filter(|(key, _)| aid_prefix(key)) {
                let aid = AgentId::from_bytes(
                    key.try_into()
                        .map_err(|_| crate::Error::msg("failed to parse agent-id from storage"))?,
                );
                out.push(aid);
            }
            drop(cursor);
            txn.commit()?;
            Ok(out)
        }
    }
}
