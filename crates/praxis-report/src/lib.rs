//! Pre-audit report generator — emits Markdown and JSON from findings and profiles.
//!
//! # Schema
//!
//! The JSON output conforms to `docs/report-schema.json` (version `"1"`).
//! Consumers can validate against that schema for CI integration.
//!
//! # Example
//!
//! ```rust
//! use e_okelloh_praxis_report::{ReportBuilder, ProgramMeta};
//!
//! let mut builder = ReportBuilder::new(ProgramMeta {
//!     name: "escrow".into(),
//!     program_id: "11111111111111111111111111111111".into(),
//!     version: "0.1.0".into(),
//! });
//!
//! let report = builder.build();
//! let _markdown = report.to_markdown();
//! let _json = report.to_json().unwrap();
//! ```
#![deny(unsafe_code)]
#![warn(missing_docs)]

mod builder;
mod markdown;

pub use builder::ReportBuilder;

use serde::{Deserialize, Serialize};

/// Metadata about the program being audited.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramMeta {
    /// Human-readable program name.
    pub name: String,
    /// On-chain program ID (base-58).
    pub program_id: String,
    /// Semver version string of the program under test.
    pub version: String,
}

/// Severity of a finding in the report.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational — low confidence or low impact.
    Info,
    /// Medium — warrants review.
    Medium,
    /// High — likely exploitable.
    High,
    /// Critical — actively exploitable with material impact.
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => f.write_str("Info"),
            Severity::Medium => f.write_str("Medium"),
            Severity::High => f.write_str("High"),
            Severity::Critical => f.write_str("Critical"),
        }
    }
}

/// A single finding included in the report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportFinding {
    /// Check ID (e.g. `AC-001`) or fuzzer finding ID.
    pub id: String,
    /// Severity classification.
    pub severity: Severity,
    /// Human-readable description of the finding.
    pub message: String,
    /// Location within the program (instruction + account, or file:line).
    pub location: Option<String>,
    /// Deterministic seed to reproduce this finding with `praxis replay`.
    pub replay_seed: Option<String>,
}

/// Per-instruction CU summary included in the report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportIxProfile {
    /// Instruction name.
    pub name: String,
    /// Average CU consumed.
    pub avg_cu: u64,
    /// Maximum CU observed.
    pub max_cu: u64,
    /// Percentage of total session CU.
    pub pct_of_total: f64,
}

/// Summary counts across severity levels.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FindingSummary {
    /// Total number of findings.
    pub total: usize,
    /// Number of critical findings.
    pub critical: usize,
    /// Number of high findings.
    pub high: usize,
    /// Number of medium findings.
    pub medium: usize,
    /// Number of info findings.
    pub info: usize,
}

/// A complete pre-audit report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// Schema version — always `"1"` for this implementation.
    pub schema_version: String,
    /// Program metadata.
    pub program: ProgramMeta,
    /// Finding counts by severity.
    pub summary: FindingSummary,
    /// All findings, sorted by severity descending.
    pub findings: Vec<ReportFinding>,
    /// Optional CU profile data.
    pub profile: Option<Vec<ReportIxProfile>>,
    /// ISO-8601 timestamp of report generation.
    pub generated_at: String,
}

impl Report {
    /// Render the report as a Markdown string.
    pub fn to_markdown(&self) -> String {
        markdown::render(self)
    }

    /// Serialize the report to a pretty-printed JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize a report from JSON.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// True if any finding is at or above `severity`.
    pub fn has_findings_at(&self, severity: &Severity) -> bool {
        self.findings.iter().any(|f| &f.severity >= severity)
    }
}
