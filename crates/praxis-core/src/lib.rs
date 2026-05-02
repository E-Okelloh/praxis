//! Core types, `Svm` trait, and `NormalIdl` for the Praxis framework.
#![deny(unsafe_code)]

pub mod idl;
pub mod mock;
pub mod svm;

pub use idl::{
    AccountConstraint, IdlType, InstructionArg, IxAccountMeta, KnownProgram,
    NormalIdl, NormalInstruction, PdaProgram, PdaRule, SeedComponent,
};
pub use mock::MockSvm;
pub use svm::{ExecResult, Svm, SvmCapabilities, SvmSnapshot};
