use std::{cell::RefMut, collections::HashMap, marker::PhantomData};

use crate::error::Result;
use crate::rt::CodeStore;
use borderless::ContractId;
use borderless_format::{Bundle, Ident, Metadata};
use borderless_kv_store::Db;
use serde::{de::DeserializeOwned, Serialize};

pub struct Registry<D: Db, F: Serialize + DeserializeOwned = Bundle> {
    code_store: CodeStore<D>,
    meta: HashMap<ContractId, (Ident, Meta)>,
    _model: PhantomData<F>,
}

impl<D, F> Registry<D, F>
where
    D: Db,
    F: Serialize + DeserializeOwned,
{
    pub fn new(store: CodeStore<D>) -> Self {
        Registry {
            code_store: store,
            meta: HashMap::new(),
            _model: PhantomData::default(),
        }
    }

    pub fn insert_contract(&mut self, contract_id: ContractId, data: Bundle) -> Result<()> {
        // split the bundle in parts();
        let (ident, meta, src) = data.parts();

        // validate the bundle

        // store metadata
        self.meta.insert(contract_id, (meta, ident));

        todo!()
    }
}

impl<D, F> AsMut<CodeStore<D>> for Registry<D, F>
where
    D: Db,
    F: Serialize + DeserializeOwned,
{
    fn as_mut(&mut self) -> &mut CodeStore<D> {
        &mut self.code_store
    }
}

impl<D, F> AsRef<CodeStore<D>> for Registry<D, F>
where
    D: Db,
    F: Serialize + DeserializeOwned,
{
    fn as_ref(&self) -> &CodeStore<D> {
        &self.code_store
    }
}
