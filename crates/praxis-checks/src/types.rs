//! Shared types for check findings.

use serde::{Deserialize, Serialize};

/// Unique identifier for a check.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CheckId(pub &'static str);

impl std::fmt::Display for CheckId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// Severity level of a finding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Informational — likely a false positive or low-confidence signal.
    Info,
    /// Medium — warrants review but not an automatic blocker.
    Medium,
    /// High — likely a genuine bug class violation.
    High,
    /// Critical — known exploitable pattern.
    Critical,
}

/// A single finding produced by a check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckFinding {
    /// Which check produced this finding.
    pub check_id: String,
    /// Severity level.
    pub severity: Severity,
    /// Human-readable description.
    pub message: String,
    /// Location within the IDL or transaction (e.g. instruction + account name).
    pub location: Option<String>,
}

/// Aggregated result of running a check.
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Check ID.
    pub check_id: String,
    /// All findings from this run (empty = pass).
    pub findings: Vec<CheckFinding>,
}

impl CheckResult {
    /// True if no findings were produced.
    pub fn passed(&self) -> bool {
        self.findings.is_empty()
    }
}
