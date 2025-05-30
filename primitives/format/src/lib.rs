//! Borderless Format
//!
//! This library contains various definitions for the borderless smart contract file fomrat
//!

use borderless_hash::Hash256;
use serde::{Deserialize, Serialize};
use serde_json;
use std::path::Path;
use std::{fs, io};
use thiserror::Error;

mod semver;

pub use semver::SemVer;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error while loading from disk - {0}")]
    Io(#[from] io::Error),

    #[error("Invalid json input - {0}")]
    Serde(#[from] serde_json::Error),
}

/// Specifies the source for some wasm module
///
/// Can be either "remote", when the code can be fetched from our remote repository,
/// or "local" - in this case the compiled module is just serialized as bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WasmSource {
    Remote { repository: String },
    Local { code: Vec<u8> },
}

// TODO: WIP - just to save some ideas
// (the name should also be different)
// -> maybe this should be part of the contract-package crate ?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmModule {
    /// hash of wasm module
    pub hash: Hash256,

    // NOTE: This ensures compatibility with old versions
    #[serde(default)]
    #[serde(with = "crate::semver::semver_as_string")]
    /// SemVer compatible version string
    pub version: SemVer,

    /// Location, where the compiled module can be obtained
    pub source: WasmSource,
}

/// Contract metadata descibe common
/// fields of the contract itsel
#[derive(Serialize, Deserialize)]
pub struct Metadata {
    /// contract name
    pub name: String,

    /// authors
    pub authors: Vec<String>,

    /// contract description
    pub description: String,

    /// Name of the application (group) that the contract is part of
    #[serde(default)]
    pub application: Option<String>,

    /// Name of the module inside the application
    #[serde(default)]
    pub app_module: Option<String>,
}

// TODO: Refine this !

/// Container conbining the Source and the
/// Metadata to provide a struct to sign
#[derive(Serialize, Deserialize)]
pub struct Contract {
    /// contract metadata
    pub meta: Metadata,

    /// contract source
    pub src: WasmModule,
}

/// Ident identify the author of this contract
#[derive(Serialize, Deserialize)]
pub struct Ident {
    /// contract signature
    pub signature: String,

    /// author public key
    pub public_key: String,
}

/// Bundle represent the top level model
/// for the smart contract file format
/// It contains the contract and the ident
/// information
#[derive(Serialize, Deserialize)]
pub struct Bundle {
    /// contract
    pub contract: Contract,

    /// ident
    pub ident: Option<Ident>,
}

impl Bundle {
    /// Load a Bundle from a JSON file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let content = fs::read_to_string(path)?;
        let bundle: Bundle = serde_json::from_str(&content)?;
        Ok(bundle)
    }

    /// split the bundle in its parts
    pub fn parts(self) -> (Option<Ident>, Metadata, Source) {
        (self.ident, self.contract.meta, self.contract.src)
    }
}
