# praxis-report

Pre-audit report generator for the Praxis Solana security framework — Markdown and JSON output.

[![Crates.io](https://img.shields.io/crates/v/e-okelloh-praxis-report.svg)](https://crates.io/crates/e-okelloh-praxis-report)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](#license)

## What this crate does

Aggregates findings from the fuzzer, check pack, and profiler into a single auditor-ready report. The report reduces audit scope by documenting exactly what was tested, what was found, and how to reproduce each finding.

## Output formats

- **Markdown** — human-readable, version-controllable, suitable for GitHub and Notion
- **JSON** — machine-readable, validates against the published Praxis finding schema

## CLI usage

```bash
praxis report
# Writes to .praxis/reports/report-<timestamp>.md and .json
```

## Programmatic usage

```rust
use praxis_report::{Report, ReportFormat};

let report = Report::builder()
    .findings(fuzz_findings)
    .check_results(check_findings)
    .profile(profile_data)
    .build();

report.write(".praxis/reports/", &[ReportFormat::Markdown, ReportFormat::Json])?;
```

## Finding severity levels

`critical` → `high` → `medium` → `low` → `informational`

Use `fail_on` in `praxis.toml` to set the minimum severity that causes a non-zero exit code in CI.

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
