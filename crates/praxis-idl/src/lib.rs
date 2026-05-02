//! IDL ingestion layer — Anchor (phase 1), Codama/Shank (phase 2).
#![deny(unsafe_code)]

pub mod anchor;
pub mod error;

pub use anchor::parse_anchor_idl;
pub use error::IdlError;
