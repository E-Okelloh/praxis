//! Error types for IDL parsing.
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IdlError {
    #[error("I/O error reading IDL file: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid program address in IDL: {0}")]
    InvalidAddress(String),

    #[error("Unsupported IDL feature: {0}")]
    Unsupported(String),
}
