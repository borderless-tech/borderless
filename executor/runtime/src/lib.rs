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

/// Sub-Database for all agent related data
pub const AGENT_SUB_DB: &str = "agent-db";

/// Sub-Database, where the wasm code is stored
pub const WASM_CODE_SUB_DB: &str = "wasm-code-db";

/// Sub-Database to store the relationship between an action and a transaction
pub const ACTION_TX_REL_SUB_DB: &str = "rel-tx-action-db";

/// Sub-Database to store the relationship between contracts and agents, and vice-versa
pub const SUBSCRIPTION_REL_SUB_DB: &str = "rel-subscription-db";

/// Sub-Database, where the ledger information is stored
pub const LEDGER_SUB_DB: &str = "ledger-db";

// TODO: Tracing vs Logging !
// We should make this toggleable via feature switch.

mod log_shim {
    #[cfg(all(feature = "tracing", not(feature = "log")))]
    pub use tracing::{debug, error, info, trace, warn};

    #[cfg(all(feature = "log", not(feature = "tracing")))]
    pub use log::{debug, error, info, trace, warn};

    // If neither logging or tracing are enabled, just send the input into the void
    #[cfg(any(
        all(not(feature = "log"), not(feature = "tracing")),
        all(feature = "log", feature = "tracing")
    ))]
    pub use crate::void as trace;

    #[cfg(any(
        all(not(feature = "log"), not(feature = "tracing")),
        all(feature = "log", feature = "tracing")
    ))]
    pub use crate::void as debug;

    #[cfg(any(
        all(not(feature = "log"), not(feature = "tracing")),
        all(feature = "log", feature = "tracing")
    ))]
    pub use crate::void as info;

    #[cfg(any(
        all(not(feature = "log"), not(feature = "tracing")),
        all(feature = "log", feature = "tracing")
    ))]
    pub use crate::void as warn;

    #[cfg(any(
        all(not(feature = "log"), not(feature = "tracing")),
        all(feature = "log", feature = "tracing")
    ))]
    pub use crate::void as error;

    #[cfg(any(
        all(not(feature = "log"), not(feature = "tracing")),
        all(feature = "log", feature = "tracing")
    ))]
    #[macro_export]
    macro_rules! void {
        ($($t:tt)*) => {{
            let _ = format_args!($($t)*);
        }};
    }

    // NOTE: We emit a compiler warning in the build.rs - which is far better than a compiler error.
    //
    // #[cfg(all(feature = "log", feature = "tracing"))]
    // compile_error!("Features `log` and `tracing` are mutually exclusive.");
}
