//! Error type for the fuzzer.
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FuzzError {
    #[error("No IDL loaded — call Ctx::with_idl() before fuzzing")]
    NoIdl,

    #[error("Instruction '{0}' not found in IDL")]
    InstructionNotFound(String),

    #[error("Finding {0} did not reproduce — seed may be stale")]
    DidNotReproduce(String),

    #[error("Invalid seed hex '{0}': {1}")]
    InvalidSeedHex(String, String),

    #[error("I/O error persisting finding: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
