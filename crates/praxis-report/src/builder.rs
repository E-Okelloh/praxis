//! `ReportBuilder` — accumulates findings and profile data, then builds a `Report`.

use crate::{
    FindingSummary, ProgramMeta, Report, ReportFinding, ReportIxProfile, Severity,
};

/// Incrementally constructs a [`Report`].
#[derive(Debug)]
pub struct ReportBuilder {
    meta: ProgramMeta,
    findings: Vec<ReportFinding>,
    profile: Option<Vec<ReportIxProfile>>,
}

impl ReportBuilder {
    /// Create a new builder for the given program.
    pub fn new(meta: ProgramMeta) -> Self {
        Self {
            meta,
            findings: Vec::new(),
            profile: None,
        }
    }

    /// Add a finding to the report.
    pub fn add_finding(&mut self, finding: ReportFinding) -> &mut Self {
        self.findings.push(finding);
        self
    }

    /// Add findings from `praxis-checks` (converts `CheckFinding` → `ReportFinding`).
    pub fn add_check_findings(
        &mut self,
        findings: impl IntoIterator<Item = praxis_checks::CheckFinding>,
    ) -> &mut Self {
        for f in findings {
            self.findings.push(ReportFinding {
                id: f.check_id,
                severity: convert_severity(f.severity),
                message: f.message,
                location: f.location,
                replay_seed: None,
            });
        }
        self
    }

    /// Add findings from the fuzzer (converts `praxis_fuzz::Finding` → `ReportFinding`).
    pub fn add_fuzz_findings(
        &mut self,
        findings: impl IntoIterator<Item = praxis_fuzz::Finding>,
    ) -> &mut Self {
        for f in findings {
            self.findings.push(ReportFinding {
                id: f.id.clone(),
                severity: Severity::High,
                message: format!(
                    "Fuzzer found invariant violation via `{}` mutation",
                    f.mutation
                ),
                location: Some(f.instruction.clone()),
                replay_seed: Some(f.id),
            });
        }
        self
    }

    /// Attach CU profile data from `praxis-profile`.
    pub fn set_profile(&mut self, report: &praxis_profile::ProfileReport) -> &mut Self {
        let ixs = report
            .instructions
            .iter()
            .map(|i| ReportIxProfile {
                name: i.label.clone(),
                avg_cu: i.avg_cu,
                max_cu: i.max_cu,
                pct_of_total: i.pct_of_total,
            })
            .collect();
        self.profile = Some(ixs);
        self
    }

    /// Build the final [`Report`].
    pub fn build(mut self) -> Report {
        // Sort findings by severity descending.
        self.findings
            .sort_by(|a, b| b.severity.cmp(&a.severity));

        let summary = summarise(&self.findings);

        Report {
            schema_version: "1".into(),
            program: self.meta,
            summary,
            findings: self.findings,
            profile: self.profile,
            generated_at: now_iso8601(),
        }
    }
}

fn summarise(findings: &[ReportFinding]) -> FindingSummary {
    let mut s = FindingSummary { total: findings.len(), ..Default::default() };
    for f in findings {
        match f.severity {
            Severity::Critical => s.critical += 1,
            Severity::High => s.high += 1,
            Severity::Medium => s.medium += 1,
            Severity::Info => s.info += 1,
        }
    }
    s
}

fn convert_severity(s: praxis_checks::Severity) -> Severity {
    match s {
        praxis_checks::Severity::Info => Severity::Info,
        praxis_checks::Severity::Medium => Severity::Medium,
        praxis_checks::Severity::High => Severity::High,
        praxis_checks::Severity::Critical => Severity::Critical,
    }
}

fn now_iso8601() -> String {
    // Use a simple formatting approach without pulling in `chrono`.
    // We read the system time and format it manually.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Convert unix timestamp to a basic ISO-8601 string (UTC).
    let (y, mo, d, h, mi, s) = unix_to_ymdhms(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn unix_to_ymdhms(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = secs % 60;
    let mins = secs / 60;
    let mi = mins % 60;
    let hours = mins / 60;
    let h = hours % 24;
    let days = hours / 24;

    // Days since 1970-01-01
    let mut year = 1970u64;
    let mut rem = days;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if rem < days_in_year {
            break;
        }
        rem -= days_in_year;
        year += 1;
    }

    let months = [31, if is_leap(year) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for &days_in_month in &months {
        if rem < days_in_month {
            break;
        }
        rem -= days_in_month;
        month += 1;
    }
    (year, month, rem + 1, h, mi, s)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}
