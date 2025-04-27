pub mod controller;
pub mod error;
pub mod http;
mod rt;

pub use error::{Error, Result};
pub use rt::contract::{Runtime, SharedRuntime};

// Forward module content
pub use rt::*;

/// Sub-Database for all contract related data
pub const CONTRACT_SUB_DB: &str = "contract-db";

/// Sub-Database, where the wasm code is stored
pub const WASM_CODE_SUB_DB: &str = "wasm-code-db";
