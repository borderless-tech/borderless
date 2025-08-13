use super::{Capabilities, PkgMeta};
use serde::{Deserialize, Serialize};

/// Manifest format for either a contract or an agent
#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    /// General information about the package - see [`PkgMeta`]
    pub package: Option<PkgMeta>,

    pub contract: Option<ContractSection>,
    pub agent: Option<Capabilities>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractSection {
    pub participants: Option<Vec<String>>,
    pub sinks: Option<Vec<SinkInfo>>,
}

/// Sink information - see [`Manifest`]
#[derive(Debug, Serialize, Deserialize)]
pub struct SinkInfo {
    /// Alias for the sink
    alias: String,
    /// Writer of the sink
    writer: String,
}
