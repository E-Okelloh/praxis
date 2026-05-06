//! Praxis CLI — Phase 2: adds `praxis profile` and `praxis check`.
#![deny(unsafe_code)]

use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "praxis",
    version,
    about = "Praxis — Solana program testing and fuzzing framework"
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run all #[invariant_test] functions in the current workspace.
    Test {
        /// Path to the workspace or crate to test (defaults to current dir).
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
        /// Extra arguments forwarded to `cargo test`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extra: Vec<String>,
    },
    /// Replay a known finding by its ID.
    Replay {
        /// Finding ID produced by the fuzzer (format: `<16-hex-seed>-<MutationName>`).
        #[arg(long)]
        seed: String,
    },
    /// Long-running adversarial fuzzing.
    Fuzz {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extra: Vec<String>,
    },
    /// CU flame-graph profiler.
    Profile {
        #[command(subcommand)]
        cmd: ProfileCmd,
    },
    /// Check pack — static + runtime bug-class checks.
    Check {
        /// Path to the Anchor IDL JSON file to analyse.
        #[arg(long)]
        idl: Option<PathBuf>,
        /// Only run checks with IDs matching this prefix (e.g. `AC`, `FD`).
        #[arg(long)]
        filter: Option<String>,
        /// Exit with non-zero code if any findings at or above this severity.
        #[arg(long, default_value = "high")]
        fail_on: String,
    },
    /// Emit Markdown + JSON pre-audit report (Phase 3 — not yet implemented).
    Report,
    /// All-in-one CI run (Phase 3 — not yet implemented).
    Ci,
    /// Scaffold `.praxis/` directory and `praxis.toml`.
    Init {
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
    },
}

#[derive(Subcommand)]
enum ProfileCmd {
    /// Generate an SVG flame graph from a stored profile JSON.
    Render {
        /// Path to a `Profiler` JSON file (written by `Profiler::to_json()`).
        #[arg(long)]
        input: PathBuf,
        /// Output SVG path (default: `profile.svg`).
        #[arg(long, default_value = "profile.svg")]
        output: PathBuf,
    },
    /// Compare two profile JSON files and print the CU delta.
    Diff {
        /// Baseline profile JSON.
        #[arg(long)]
        baseline: PathBuf,
        /// New profile JSON.
        #[arg(long)]
        new: PathBuf,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("praxis=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Cmd::Test { path, extra } => cmd_test(path, extra),
        Cmd::Replay { seed } => cmd_replay(seed),
        Cmd::Fuzz { extra } => cmd_fuzz(extra),
        Cmd::Init { path } => cmd_init(path),
        Cmd::Profile { cmd } => cmd_profile(cmd),
        Cmd::Check { idl, filter, fail_on } => cmd_check(idl, filter, fail_on),
        Cmd::Report | Cmd::Ci => {
            bail!("This subcommand is not yet implemented (Phase 3).")
        }
    }
}

fn cmd_test(path: PathBuf, extra: Vec<String>) -> Result<()> {
    let status = Command::new("cargo")
        .arg("test")
        .args(&extra)
        .current_dir(&path)
        .status()
        .context("failed to spawn `cargo test`")?;

    if !status.success() {
        bail!("`cargo test` exited with status {}", status);
    }
    Ok(())
}

fn cmd_replay(seed_id: String) -> Result<()> {
    let finding_path = PathBuf::from(format!(".praxis/findings/{}.json", seed_id));

    if !finding_path.exists() {
        bail!(
            "No finding file at {}. Run `praxis fuzz` first to generate findings.",
            finding_path.display()
        );
    }

    let raw = std::fs::read_to_string(&finding_path)
        .with_context(|| format!("reading {}", finding_path.display()))?;
    let finding: serde_json::Value = serde_json::from_str(&raw).context("parsing finding JSON")?;

    println!("=== Praxis Finding ===");
    println!("{}", serde_json::to_string_pretty(&finding)?);
    println!();
    println!(
        "To reproduce: {}",
        finding["replay_cmd"]
            .as_str()
            .unwrap_or("praxis replay --seed <id>")
    );
    Ok(())
}

fn cmd_fuzz(extra: Vec<String>) -> Result<()> {
    let status = Command::new("cargo")
        .arg("test")
        .args(&extra)
        .env("PRAXIS_FUZZ", "1")
        .status()
        .context("failed to spawn `cargo test`")?;

    if !status.success() {
        bail!("`cargo test` (fuzz mode) exited with status {}", status);
    }
    Ok(())
}

fn cmd_init(path: PathBuf) -> Result<()> {
    let praxis_dir = path.join(".praxis");
    let findings_dir = praxis_dir.join("findings");
    std::fs::create_dir_all(&findings_dir)
        .with_context(|| format!("creating {}", findings_dir.display()))?;

    let toml_path = path.join("praxis.toml");
    if !toml_path.exists() {
        std::fs::write(&toml_path, include_str!("../default_praxis.toml"))
            .with_context(|| format!("writing {}", toml_path.display()))?;
        println!("Created {}", toml_path.display());
    } else {
        println!("{} already exists, skipping.", toml_path.display());
    }

    println!("Created {}", findings_dir.display());
    println!("Praxis initialised. Edit praxis.toml to configure your program.");
    Ok(())
}

fn cmd_profile(cmd: ProfileCmd) -> Result<()> {
    use praxis_profile::Profiler;

    match cmd {
        ProfileCmd::Render { input, output } => {
            let json = std::fs::read_to_string(&input)
                .with_context(|| format!("reading profile from {}", input.display()))?;
            let profiler = Profiler::from_json(&json)
                .map_err(|e| anyhow::anyhow!("invalid profile JSON: {e}"))?;

            let svg = profiler
                .flame_graph_svg()
                .map_err(|e| anyhow::anyhow!("flame graph rendering failed: {e}"))?;

            std::fs::write(&output, &svg)
                .with_context(|| format!("writing SVG to {}", output.display()))?;

            println!("Flame graph written to {}", output.display());
            println!(
                "  Total CU: {}   Samples: {}",
                profiler.total_cu(),
                profiler.samples().len()
            );

            let report = profiler.report();
            println!("\n  Top instructions by CU:");
            for ix in report.instructions.iter().take(10) {
                println!(
                    "    {:30}  avg={:>8} CU   total={:>10} CU   ({:.1}%)",
                    ix.label, ix.avg_cu, ix.total_cu, ix.pct_of_total
                );
            }
            Ok(())
        }

        ProfileCmd::Diff { baseline, new } => {
            let baseline_json = std::fs::read_to_string(&baseline)
                .with_context(|| format!("reading baseline profile from {}", baseline.display()))?;
            let new_json = std::fs::read_to_string(&new)
                .with_context(|| format!("reading new profile from {}", new.display()))?;

            let baseline_profiler = Profiler::from_json(&baseline_json)
                .map_err(|e| anyhow::anyhow!("invalid baseline JSON: {e}"))?;
            let new_profiler = Profiler::from_json(&new_json)
                .map_err(|e| anyhow::anyhow!("invalid new profile JSON: {e}"))?;

            let diff = new_profiler.diff(&baseline_profiler);

            let sign = if diff.total_delta >= 0 { "+" } else { "" };
            println!(
                "CU delta: {sign}{} ({} → {})",
                diff.total_delta, diff.baseline_total_cu, diff.new_total_cu
            );
            println!("\n  Per-instruction deltas (sorted by absolute change):");
            for d in diff.instructions.iter().take(20) {
                let sign = if d.delta >= 0 { "+" } else { "" };
                println!(
                    "    {:30}  {sign}{:>6} CU   ({:.1}%)",
                    d.label, d.delta, d.pct_change
                );
            }
            Ok(())
        }
    }
}

fn cmd_check(idl_path: Option<PathBuf>, filter: Option<String>, fail_on: String) -> Result<()> {
    use praxis_checks::{check_ac_001, check_ac_002, Severity};
    use praxis_idl::parse_anchor_idl;

    let idl_path = idl_path
        .or_else(|| {
            // Try to find IDL from praxis.toml.
            let toml_raw = std::fs::read_to_string("praxis.toml").ok()?;
            let table: toml::Value = toml_raw.parse().ok()?;
            let idl_str = table.get("program")?.get("idl")?.as_str()?.to_owned();
            Some(PathBuf::from(idl_str))
        })
        .context("No IDL path provided and could not read praxis.toml [program].idl")?;

    let idl = parse_anchor_idl(&idl_path)
        .with_context(|| format!("parsing IDL from {}", idl_path.display()))?;

    let mut all_findings = Vec::new();
    all_findings.extend(check_ac_001(&idl));
    all_findings.extend(check_ac_002(&idl));

    // Apply ID filter if specified.
    if let Some(ref prefix) = filter {
        all_findings.retain(|f| f.check_id.starts_with(prefix.as_str()));
    }

    let fail_severity = match fail_on.to_lowercase().as_str() {
        "info" => Severity::Info,
        "medium" => Severity::Medium,
        "high" => Severity::High,
        "critical" => Severity::Critical,
        other => bail!("Unknown severity `{other}`. Use: info, medium, high, critical"),
    };

    if all_findings.is_empty() {
        println!("All checks passed. No findings.");
        return Ok(());
    }

    let mut should_fail = false;
    for f in &all_findings {
        let sev_str = format!("{:?}", f.severity).to_uppercase();
        let loc = f.location.as_deref().unwrap_or("-");
        println!("[{}] {} — {} ({})", f.check_id, sev_str, f.message, loc);
        if f.severity >= fail_severity {
            should_fail = true;
        }
    }

    println!("\n{} finding(s).", all_findings.len());

    if should_fail {
        bail!("One or more findings at or above severity `{fail_on}`.");
    }
    Ok(())
}
