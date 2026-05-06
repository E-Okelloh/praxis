//! Check pack — static and runtime checks for common Solana bug classes.
//!
//! # Check IDs
//!
//! | ID | Description |
//! |----|-------------|
//! | `AC-001` | Every authority parameter has a signer constraint |
//! | `AC-002` | Every deserialised account has an explicit owner check |
//! | `CPI-001` | All CPIs target whitelisted program IDs |
//! | `FD-001` | Protocol invariants hold across N skipped slots |
//! | `FD-002` | Pyth/Switchboard `last_update_slot` ≤ N slots old |
//! | `FD-003` | Pyth confidence interval rejected above threshold |
//! | `T22-001` | Transfer Hook does not CPI back into same mint |
//! | `T22-002` | All `ExtraAccountMetaList` seeds validated |
//! | `T22-003` | ZK proof inputs match expected ciphertexts |
#![deny(unsafe_code)]
#![warn(missing_docs)]

mod ac;
mod cpi;
mod fd;
mod t22;
mod types;

pub use ac::{check_ac_001, check_ac_002};
pub use cpi::{check_cpi_001, report_cpi_finding};
pub use fd::{
    check_fd_001, check_fd_002, check_fd_002_staleness, check_fd_003, check_fd_003_confidence,
    report_fd_001_violation,
};
pub use t22::{
    check_t22_001, check_t22_002, check_t22_003, report_t22_001_reentrant_cpi,
    report_t22_003_zk_mismatch,
};
pub use types::{CheckFinding, CheckId, CheckResult, Severity};

use praxis_core::NormalIdl;

/// Run all static checks against a [`NormalIdl`] and return every finding.
pub fn run_static_checks(idl: &NormalIdl) -> Vec<CheckFinding> {
    let mut findings = Vec::new();
    findings.extend(check_ac_001(idl));
    findings.extend(check_ac_002(idl));
    findings
}
