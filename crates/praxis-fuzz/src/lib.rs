//! Invariant fuzzer engine — `Ctx`, `Finding`, and the fuzz loop.
#![deny(unsafe_code)]

pub mod ctx;
pub mod engine;
pub mod error;
pub mod finding;

pub use ctx::Ctx;
pub use error::FuzzError;
pub use finding::Finding;
