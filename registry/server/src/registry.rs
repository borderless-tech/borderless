use crate::error::Error;
use anyhow::Result;
use async_trait::async_trait;
use bincode;
use borderless_format::{
    pkg::{InsertPkg, Pkg},
    registry::ContractService,
};
use borderless_hash::Hash256;
use borderless_kv_store::{Db, RawRead, RawWrite, RoCursor, RoTx, Tx};

const PKG_SUB_DB: &str = "pkg-sub-db";
const CONTRACT_SUB_DB: &str = "contract-sub-db";

#[derive(Clone)]
pub struct ContractRegistry<S: Db> {
    db: S,
}

impl<S> ContractRegistry<S>
where
    S: Db,
{
    pub fn new(db: S) -> Result<Self> {
        // Create sub database for contract and pkg
        // information
        db.create_sub_db(CONTRACT_SUB_DB)?;
        db.create_sub_db(PKG_SUB_DB)?;
        Ok(ContractRegistry { db })
    }
}

#[async_trait]
impl<S> ContractService for ContractRegistry<S>
where
    S: Db,
{
    type Error = Error;

    async fn get_contract(&self, hash: Hash256) -> Result<Vec<u8>, Self::Error> {
        let db_ptr = self.db.open_sub_db(CONTRACT_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;
        let buf = txn
            .read(&db_ptr, &hash)?
            .ok_or(Error::NoContract(hash))?
            .to_vec();

        Ok(buf)
    }

    async fn list_pkg(&self) -> Result<Vec<String>, Self::Error> {
        let db_ptr = self.db.open_sub_db(PKG_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;
        let mut cursor = txn.ro_cursor(&db_ptr)?;

        let mut pkgs = Vec::new();
        for (_, buf) in cursor.iter() {
            let pkg: Pkg = bincode::deserialize(&buf)?;
            let name = format!("{}:{}", pkg.name, pkg.version.to_string());
            pkgs.push(name);
        }

        Ok(pkgs)
    }

    async fn create_pkg(&self, new_pkg: InsertPkg) -> Result<(), Self::Error> {
        let id = format!("{}:{}", &new_pkg.pkg.name, &new_pkg.pkg.version.to_string());
        let key = Hash256::digest(&id);

        // check for dublicated
        {
            let db_ptr = self.db.open_sub_db(PKG_SUB_DB)?;

            let txn = self.db.begin_rw_txn()?;
            let buf = txn.read(&db_ptr, &key)?;

            if buf.is_some() {
                return Err(Error::Dublicated(key));
            }

            let mut txn = self.db.begin_rw_txn()?;

            let buf = bincode::serialize(&new_pkg.pkg)?;
            txn.write(&db_ptr, &key, &buf)?;
            txn.commit()?;
        }

        // add contract
        {
            let db_ptr = self.db.open_sub_db(CONTRACT_SUB_DB)?;

            let mut txn = self.db.begin_rw_txn()?;
            txn.write(&db_ptr, &key, &new_pkg.contract)?;
            txn.commit()?;
        }

        Ok(())
    }

    async fn read_pkg(&self, name: String) -> Result<Pkg, Self::Error> {
        let db_ptr = self.db.open_sub_db(PKG_SUB_DB)?;
        let txn = self.db.begin_ro_txn()?;

        let key = Hash256::digest(&name);
        let buf = txn.read(&db_ptr, &key)?.ok_or(Error::NoPkg(key))?;
        let pkg = bincode::deserialize(&buf)?;
        Ok(pkg)
    }
}
