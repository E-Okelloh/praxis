//! CPI-001 — CPI target whitelist check.

use praxis_core::NormalIdl;

use crate::types::CheckFinding;

/// CPI-001: Placeholder for runtime CPI check.
///
/// This check is enforced at runtime by the fuzzer (not statically from the IDL),
/// because CPI targets are only known after instruction execution.  The fuzzer
/// calls [`report_cpi_finding`] when it detects a CPI to a non-whitelisted program.
///
/// Static analysis over the IDL returns an empty vec (no false positives).
pub fn check_cpi_001(_idl: &NormalIdl) -> Vec<CheckFinding> {
    // CPI-001 is a runtime check only; no static findings.
    vec![]
}

/// Called by the fuzzer runtime when a CPI to an unexpected program is observed.
pub fn report_cpi_finding(ix_name: &str, cpi_target: &str) -> CheckFinding {
    CheckFinding {
        check_id: "CPI-001".into(),
        severity: crate::types::Severity::High,
        message: format!(
            "Instruction `{ix_name}` CPI'd to unexpected program `{cpi_target}` — possible arbitrary CPI"
        ),
        location: Some(ix_name.to_owned()),
    }
}
