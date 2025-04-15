use borderless_sdk::ContractId;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    // --- High-level errors
    #[error("storage error - {0}")]
    Db(#[from] borderless_kv_store::Error),

    #[error("encoding error (binary) - {0}")]
    BinaryEncoding(#[from] postcard::Error),

    #[error("encoding error (json) - {0}")]
    JsonEncoding(#[from] serde_json::Error),

    #[error("wasmtime error - {0}")]
    Wasm(#[from] wasmtime::Error),

    // --- Module related errors
    #[error("exported function '{func}' has invalid type")]
    InvalidFuncType { func: &'static str },

    #[error("exported item '{func}' must be a function")]
    InvalidExport { func: &'static str },

    #[error("module is missing required export '{func}'")]
    MissingExport { func: &'static str },

    // --- Runtime errors
    #[error("contract is not instantiated cid={cid}")]
    MissingContract { cid: ContractId },

    // --- Register errors
    #[error("missing required value '{0}' in register")]
    MissingRegisterValue(&'static str),

    #[error("invalid value in register '{register}' - expected {expected_type}")]
    InvalidRegisterValue {
        register: &'static str,
        expected_type: &'static str,
    },

    // --- VmState errors
    #[error("{0}")]
    Msg(&'static str),
}

// pub struct Error {
//     kind: ErrorKind,
// }

// pub(crate) enum ErrorKind {

// }
