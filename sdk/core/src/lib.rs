pub mod collections;
pub mod contract;
pub mod log;
pub mod storage;

pub use anyhow::{anyhow as new_error, Context, Error, Result};

pub mod serialize {
    pub use serde_json::from_value;
}

/// This module is **not** part of the public API.
/// It exists, because the procedural macros and some internal implementations (like the contract runtime) rely on it.
///
/// You probably don't want to use this directly.
#[doc(hidden)]
#[path = "private.rs"]
pub mod __private;

// Re-export all id-types at top-level
pub use borderless_id_types::*;

// Re-export entire hash crate
pub use borderless_hash as hash;

// Re-export some parts of the http module
pub mod http {
    pub use crate::__private::http::{Method, Request, Response};
}
