use axum::{http::StatusCode, response::IntoResponse, Json};
use borderless_hash::Hash256;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Bincode error - {0}")]
    Bincode(#[from] bincode::Error),
    #[error("Storage error - {0}")]
    Storage(#[from] borderless_kv_store::Error),
    #[error("Duplicated Key: {0}")]
    Dublicated(Hash256),
    #[error("No entry in storage for key: {0}")]
    NoPkg(Hash256),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match &self {
            Error::Bincode(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            Error::Storage(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            Error::Dublicated(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            Error::NoPkg(_) => (StatusCode::NOT_FOUND, self.to_string()),
        };

        let body = Json(json!({
            "error": {
                "message": error_message,
                "status": status.as_u16()
            }
        }));

        (status, body).into_response()
    }
}
