pub mod db;
pub mod error;

#[cfg(feature = "http")]
pub mod http;

mod rt;

pub use error::{Error, Result};

#[cfg(feature = "contracts")]
pub use rt::contract::{Runtime, SharedRuntime};

// Forward module content
#[cfg(any(feature = "contracts", feature = "agents"))]
pub use rt::*;

/// Sub-Database for all contract related data
pub const CONTRACT_SUB_DB: &str = "contract-db";

/// Sub-Database, where the wasm code is stored
pub const WASM_CODE_SUB_DB: &str = "wasm-code-db";
