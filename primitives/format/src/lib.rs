//! Borderless Format
//!
//! This library contains various definitions for the borderless smart contract file fomrat
//!

use borderless_hash::Hash256;
use serde::{Deserialize, Serialize};

/// Source contains information about
/// wasm module as base64 and metadata
/// like sdk and compiler version and the
/// hash of the wasm
#[derive(Serialize, Deserialize)]
pub struct Source {
    /// hash of wasm module
    hash: Hash256,

    /// sdk version
    version: String,

    /// compiler version
    compiler: String,

    /// wasm module
    wasm: String,
}

/// Contract metadata descibe common
/// fields of the contract itsel
#[derive(Serialize, Deserialize)]
pub struct Metadata {
    /// decentralized identifier
    did: String,

    /// contract name
    name: String,

    /// contract version
    version: String,

    /// authors
    authors: Vec<String>,

    /// contract description
    description: String,
}

/// Container conbining the Source and the
/// Metadata to provide a struct to sign
#[derive(Serialize, Deserialize)]
pub struct Contract {
    /// contract metadata
    meta: Metadata,

    /// contract source
    src: Source,
}

/// Ident identify the author of this contract
#[derive(Serialize, Deserialize)]
pub struct Ident {
    /// contract signature
    signature: String,

    /// author public key
    public_key: String,
}

/// Bundle represent the top level model
/// for the smart contract file format
/// It contains the contract and the ident
/// information
#[derive(Serialize, Deserialize)]
pub struct Bundle {
    /// contract
    contract: Contract,

    /// ident
    ident: Ident,
}
