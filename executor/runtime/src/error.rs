use borderless::{AgentId, ContractId};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
#[error(transparent)]
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    pub fn msg(msg: impl AsRef<str>) -> Self {
        ErrorKind::Msg(msg.as_ref().to_string()).into()
    }
}

impl From<ErrorKind> for Error {
    fn from(value: ErrorKind) -> Self {
        Error { kind: value }
    }
}

impl From<borderless_kv_store::Error> for Error {
    fn from(value: borderless_kv_store::Error) -> Self {
        ErrorKind::from(value).into()
    }
}

impl From<postcard::Error> for Error {
    fn from(value: postcard::Error) -> Self {
        ErrorKind::from(value).into()
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        ErrorKind::from(value).into()
    }
}

impl From<wasmtime::Error> for Error {
    fn from(value: wasmtime::Error) -> Self {
        ErrorKind::from(value).into()
    }
}

#[derive(Debug, Error)]
#[allow(dead_code)]
pub(crate) enum ErrorKind {
    // --- High-level errors
    /// Storage errors
    #[error("storage error - {0}")]
    Db(#[from] borderless_kv_store::Error),

    /// Binary-Encoding (postcard) related errors
    #[error("encoding error (binary) - {0}")]
    BinaryEncoding(#[from] postcard::Error),

    /// Json-Encoding related errors
    #[error("encoding error (json) - {0}")]
    JsonEncoding(#[from] serde_json::Error),

    /// Wasmtime related errors
    #[error("wasmtime error - {0}")]
    Wasm(#[from] wasmtime::Error),

    // --- Module related errors
    /// Module function has an incorrect type
    #[error("exported function '{func}' has invalid type")]
    InvalidFuncType { func: &'static str },

    /// Module export is not a function
    #[error("exported item '{func}' must be a function")]
    InvalidExport { func: &'static str },

    /// Module is missing a required export
    #[error("module is missing required export '{func}'")]
    MissingExport { func: &'static str },

    /// Contract has not been instantiated and was not found in contract storage
    // --- Runtime errors
    #[error("contract is not instantiated cid={cid}")]
    MissingContract { cid: ContractId },

    #[error("contract is not instantiated aid={aid}")]
    MissingAgent { aid: AgentId },

    #[error("contract is revoked and cannot process transactions cid={cid}")]
    RevokedContract { cid: ContractId },

    /// Missing required value in register
    // --- Register errors
    #[error("missing required value '{0}' in register")]
    MissingRegisterValue(&'static str),

    /// The value read from a register could not be parsed into the expected type
    #[error("invalid value in register '{register}' - expected {expected_type}")]
    InvalidRegisterValue {
        register: &'static str,
        expected_type: &'static str,
    },

    // --- VmState errors
    #[error("running entity in VmState is marked 'immutable' - cannot mutate state or storage")]
    Immutable,

    #[error("no active entity in VmState")]
    NoActiveEntity,

    /// Generic error message - useful for communicating more complicated errors
    #[error("{0}")]
    Msg(String),
}
