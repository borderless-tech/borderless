use crate::pkg::{InsertPkg, Pkg};
use async_trait::async_trait;
use borderless_hash::Hash256;

#[async_trait]
pub trait ContractService {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn get_contract(&self, hash: Hash256) -> Result<Vec<u8>, Self::Error>;
    async fn list_pkg(&self) -> Result<Vec<String>, Self::Error>;
    async fn create_pkg(&self, pkg: InsertPkg) -> Result<(), Self::Error>;
    async fn read_pkg(&self, name: String) -> Result<Pkg, Self::Error>;
}
