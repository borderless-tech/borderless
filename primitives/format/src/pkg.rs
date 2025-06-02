use crate::SemVer;
use borderless_hash::Hash256;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PackageType {
    Contract,
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pkg {
    pub pkg_type: PackageType,
    pub name: String,
    pub description: String,
    pub hash: Hash256,
    #[serde(default)]
    #[serde(with = "crate::semver::semver_as_string")]
    pub version: SemVer,
    pub author: Author,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertPkg {
    pub pkg: Pkg,
    #[serde(with = "serde_bytes")]
    pub contract: Vec<u8>,
}
