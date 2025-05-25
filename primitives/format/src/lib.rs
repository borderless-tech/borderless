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

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error while loading from disk - {0}")]
    Io(#[from] io::Error),

    #[error("Invalid json input - {0}")]
    Serde(#[from] serde_json::Error),
}

/// Source contains information about
/// wasm module as base64 and metadata
/// like sdk and compiler version and the
/// hash of the wasm
#[derive(Serialize, Deserialize)]
pub struct Source {
    /// hash of wasm module
    pub hash: Hash256,

    /// sdk version
    pub version: String,

    /// compiler version
    pub compiler: String,

    /// wasm module
    pub wasm: String,
}

/// Contract metadata descibe common
/// fields of the contract itsel
#[derive(Serialize, Deserialize)]
pub struct Metadata {
    /// decentralized identifier
    pub did: String,

    /// contract name
    pub name: String,

    /// contract version
    pub version: String,

    /// authors
    pub authors: Vec<String>,

    /// contract description
    pub description: String,
}

/// Container conbining the Source and the
/// Metadata to provide a struct to sign
#[derive(Serialize, Deserialize)]
pub struct Contract {
    /// contract metadata
    pub meta: Metadata,

    /// contract source
    pub src: Source,
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
