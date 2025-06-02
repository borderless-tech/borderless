pub mod db;
pub mod error;

#[cfg(feature = "http")]
pub mod http;

mod rt;

pub use error::{Error, Result};

#[cfg(feature = "contracts")]
pub use rt::contract::{
    MutLock as ContractLock, Runtime as ContractRuntime, SharedRuntime as SharedContractRuntime,
};

#[cfg(feature = "agents")]
pub use rt::agent::{
    MutLock as AgentLock, Runtime as AgentRuntime, SharedRuntime as SharedAgentRuntime,
};

// Forward module content
#[cfg(any(feature = "contracts", feature = "agents"))]
pub use rt::*;

/// Sub-Database for all contract related data
pub const CONTRACT_SUB_DB: &str = "contract-db";

// TODO: Do we want two sub-dbs ??
/// Sub-Database for all agent related data
pub const AGENT_SUB_DB: &str = "agent-db";

/// Sub-Database, where the wasm code is stored
pub const WASM_CODE_SUB_DB: &str = "wasm-code-db";
