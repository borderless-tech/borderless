#[cfg(feature = "contracts")]
pub mod contract;

#[cfg(feature = "agents")]
pub mod agent;

#[cfg(any(feature = "contracts", feature = "agents"))]
mod vm;

#[cfg(any(feature = "contracts", feature = "agents"))]
pub use code_store::CodeStore;

#[cfg(any(feature = "contracts", feature = "agents"))]
pub mod factory {
    use std::num::NonZeroUsize;

    use super::CodeStore;
    use crate::{AgentLock, AgentRuntime, ContractLock, ContractRuntime, Result};
    use borderless_kv_store::Db;

    /// Create new runtimes with shared code-store, cache and lock.
    ///
    /// This factory is capable of spawning agent runtimes and contract runtimes.
    /// The agent and contract runtimes spawned by this factory have a shared [`CodeStore`],
    /// but their individual execution locks (there is one lock for all agents and one lock for all contracts).
    pub struct RtFactory<'a, S: Db> {
        code_store: Option<CodeStore<S>>,
        #[cfg(feature = "contracts")]
        lck_contract: Option<ContractLock>,
        #[cfg(feature = "agents")]
        lck_agent: Option<AgentLock>,
        db: &'a S,
    }

    impl<'a, S: Db> RtFactory<'a, S> {
        /// Creates a new factory
        pub fn new(db: &'a S) -> Self {
            Self {
                code_store: None,
                #[cfg(feature = "agents")]
                lck_agent: None,
                #[cfg(feature = "contracts")]
                lck_contract: None,
                db,
            }
        }

        /// Creates a new over an existing code store
        pub fn with_store(db: &'a S, code_store: CodeStore<S>) -> Self {
            Self {
                code_store: Some(code_store),
                #[cfg(feature = "agents")]
                lck_agent: None,
                #[cfg(feature = "contracts")]
                lck_contract: None,
                db,
            }
        }

        /// Sets the cache size and initializes the code-store
        ///
        /// # Panic
        ///
        /// This function will panic, if the code-store has already been initialized.
        pub fn set_cache_size(&mut self, cache_size: NonZeroUsize) -> Result<()> {
            match &mut self.code_store {
                Some(_cache) => {
                    panic!("cannot initialize cache twice")
                }
                None => {
                    self.code_store = Some(CodeStore::with_cache_size(self.db, cache_size)?);
                }
            }
            Ok(())
        }

        /// Creates a new contract runtime
        #[cfg(feature = "contracts")]
        pub fn spawn_contract_rt(&mut self) -> Result<ContractRuntime<S>> {
            if self.code_store.is_none() {
                self.code_store = Some(CodeStore::new(self.db)?);
            }
            let code_store = self.code_store.as_ref().unwrap();

            if self.lck_contract.is_none() {
                self.lck_contract = Some(ContractLock::default());
            }
            let lock = self.lck_contract.as_ref().unwrap();

            Ok(ContractRuntime::new(
                self.db,
                code_store.clone(),
                lock.clone(),
            )?)
        }

        /// Creates a new agent runtime
        #[cfg(feature = "agents")]
        pub fn spawn_agent_rt(&mut self) -> Result<AgentRuntime<S>> {
            if self.code_store.is_none() {
                self.code_store = Some(CodeStore::new(self.db)?);
            }
            let code_store = self.code_store.as_ref().unwrap();

            if self.lck_agent.is_none() {
                self.lck_agent = Some(AgentLock::default());
            }
            let lock = self.lck_agent.as_ref().unwrap();

            Ok(AgentRuntime::new(
                self.db,
                code_store.clone(),
                lock.clone(),
            )?)
        }
    }
}

#[cfg(feature = "code-store")]
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
        cache: Arc<Mutex<LruCache<Id, Instance, ahash::RandomState>>>,
    }

    impl<S: Db> CodeStore<S> {
        pub fn new(db: &S) -> Result<Self> {
            Self::with_cache_size(db, NonZeroUsize::new(16).unwrap())
        }

        pub fn with_cache_size(db: &S, cache_size: NonZeroUsize) -> Result<Self> {
            let _db_ptr = db.create_sub_db(WASM_CODE_SUB_DB)?;
            let cache = LruCache::with_hasher(cache_size, ahash::RandomState::default());
            Ok(Self {
                db: db.clone(),
                cache: Arc::new(Mutex::new(cache)),
            })
        }

        pub fn insert_contract(&self, cid: ContractId, module: Module) -> Result<()> {
            let module_bytes = module.serialize()?;
            let db_ptr = self.db.open_sub_db(WASM_CODE_SUB_DB)?;
            let mut txn = self.db.begin_rw_txn()?;
            txn.write(&db_ptr, &cid, &module_bytes)?;
            txn.commit()?;
            Ok(())
        }

        pub fn insert_swagent(&self, aid: AgentId, module: Module) -> Result<()> {
            let module_bytes = module.serialize()?;
            let db_ptr = self.db.open_sub_db(WASM_CODE_SUB_DB)?;
            let mut txn = self.db.begin_rw_txn()?;
            txn.write(&db_ptr, &aid, &module_bytes)?;
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
            let db_ptr = self.db.open_sub_db(WASM_CODE_SUB_DB)?;
            let txn = self.db.begin_ro_txn()?;
            let module_bytes = txn.read(&db_ptr, cid)?;
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
            let db_ptr = self.db.open_sub_db(WASM_CODE_SUB_DB)?;
            let txn = self.db.begin_ro_txn()?;
            let module_bytes = txn.read(&db_ptr, &key)?;
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
            let db_ptr = self.db.open_sub_db(WASM_CODE_SUB_DB)?;
            let txn = self.db.begin_ro_txn()?;
            let mut cursor = txn.ro_cursor(&db_ptr)?;
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
            let db_ptr = self.db.open_sub_db(WASM_CODE_SUB_DB)?;
            let txn = self.db.begin_ro_txn()?;
            let mut cursor = txn.ro_cursor(&db_ptr)?;
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
