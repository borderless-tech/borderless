use borderless_hash::Hash256;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Contract not found for Hash: {0}")]
    NoContract(Hash256),
    #[error("HTTP client error - {0}")]
    Http(#[from] reqwest::Error),
    #[error("Json error - {0}")]
    Json(#[from] serde_json::Error),
    #[error("Dummy Error")]
    Dummy,
}
