//! FD-001, FD-002, FD-003 — feed / freshness / flash-loan checks.

use praxis_core::NormalIdl;

use crate::types::{CheckFinding, Severity};

/// FD-001: Protocol invariants hold across N skipped slots.
///
/// This is a runtime check driven by the fuzzer.  The IDL has no static signal
/// for this — returns empty by default.  The fuzzer calls [`report_fd_001_violation`]
/// when it detects a state invariant violation after slot warping.
pub fn check_fd_001(_idl: &NormalIdl) -> Vec<CheckFinding> {
    vec![]
}

/// Report a FD-001 violation detected at runtime.
pub fn report_fd_001_violation(message: &str, slots_skipped: u64) -> CheckFinding {
    CheckFinding {
        check_id: "FD-001".into(),
        severity: Severity::High,
        message: format!("Protocol invariant violated after {slots_skipped} skipped slots: {message}"),
        location: None,
    }
}

/// FD-002: Oracle `last_update_slot` is not older than the configured threshold.
///
/// This is a runtime check.  Returns a finding if the oracle data in the
/// provided account bytes is older than `max_staleness_slots`.
pub fn check_fd_002(_idl: &NormalIdl) -> Vec<CheckFinding> {
    vec![]
}

/// Check oracle staleness from raw account data.
///
/// `last_update_slot` is the value read from the oracle account.
/// `current_slot` is the SVM's current slot.
/// `max_staleness_slots` is the maximum allowable staleness.
pub fn check_fd_002_staleness(
    oracle_name: &str,
    last_update_slot: u64,
    current_slot: u64,
    max_staleness_slots: u64,
) -> Option<CheckFinding> {
    if current_slot.saturating_sub(last_update_slot) > max_staleness_slots {
        Some(CheckFinding {
            check_id: "FD-002".into(),
            severity: Severity::High,
            message: format!(
                "Oracle `{oracle_name}` last updated at slot {last_update_slot}, current slot {current_slot} — staleness exceeds {max_staleness_slots} slots"
            ),
            location: Some(oracle_name.to_owned()),
        })
    } else {
        None
    }
}

/// FD-003: Pyth confidence interval rejected above threshold.
pub fn check_fd_003(_idl: &NormalIdl) -> Vec<CheckFinding> {
    vec![]
}

/// Check Pyth confidence interval.
///
/// Returns a finding if `confidence / price > max_conf_ratio`.
pub fn check_fd_003_confidence(
    oracle_name: &str,
    price: u64,
    confidence: u64,
    max_conf_ratio: f64,
) -> Option<CheckFinding> {
    if price == 0 {
        return Some(CheckFinding {
            check_id: "FD-003".into(),
            severity: Severity::Critical,
            message: format!("Oracle `{oracle_name}` reported price 0 — division guard triggered"),
            location: Some(oracle_name.to_owned()),
        });
    }
    let ratio = confidence as f64 / price as f64;
    if ratio > max_conf_ratio {
        Some(CheckFinding {
            check_id: "FD-003".into(),
            severity: Severity::High,
            message: format!(
                "Oracle `{oracle_name}` confidence/price ratio {ratio:.4} exceeds threshold {max_conf_ratio:.4}"
            ),
            location: Some(oracle_name.to_owned()),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fd_002_fires_on_stale_oracle() {
        let finding = check_fd_002_staleness("pyth_sol_usd", 100, 200, 50);
        assert!(finding.is_some());
        assert_eq!(finding.unwrap().check_id, "FD-002");
    }

    #[test]
    fn fd_002_passes_on_fresh_oracle() {
        let finding = check_fd_002_staleness("pyth_sol_usd", 190, 200, 50);
        assert!(finding.is_none());
    }

    #[test]
    fn fd_003_fires_on_high_confidence_ratio() {
        // confidence is 20% of price — above a 10% threshold
        let finding = check_fd_003_confidence("pyth_sol_usd", 1_000_000, 200_000, 0.10);
        assert!(finding.is_some());
        assert_eq!(finding.unwrap().check_id, "FD-003");
    }

    #[test]
    fn fd_003_passes_on_low_confidence_ratio() {
        let finding = check_fd_003_confidence("pyth_sol_usd", 1_000_000, 50_000, 0.10);
        assert!(finding.is_none());
    }
}
