//! Error type for the profiler crate.
#![allow(missing_docs)]

use thiserror::Error;

/// Errors that can occur during profiling or report generation.
#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("flame graph rendering failed: {0}")]
    FlameGraph(String),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
