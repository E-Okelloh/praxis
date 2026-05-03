//! Praxis CLI — Phase 1: `praxis test` and `praxis replay`.
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
    /// Long-running adversarial fuzzing (Phase 1 basic).
    Fuzz {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extra: Vec<String>,
    },
    /// CU flame-graph profiler (Phase 2 — not yet implemented).
    Profile,
    /// Check pack — fast static + runtime checks (Phase 2 — not yet implemented).
    Check,
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Test { path, extra } => cmd_test(path, extra),
        Cmd::Replay { seed } => cmd_replay(seed),
        Cmd::Fuzz { extra } => cmd_fuzz(extra),
        Cmd::Init { path } => cmd_init(path),
        Cmd::Profile | Cmd::Check | Cmd::Report | Cmd::Ci => {
            bail!("This subcommand is not yet implemented (Phase 2/3).")
        }
    }
}

fn cmd_test(path: PathBuf, extra: Vec<String>) -> Result<()> {
    // Delegate to `cargo test` — the #[invariant_test] macro expands to #[test],
    // so the normal test runner discovers all invariant tests automatically.
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
    // Finding IDs are stored in `.praxis/findings/<id>.json`.
    // We print the finding and give the user the reproduction command they need.
    let finding_path = PathBuf::from(format!(".praxis/findings/{}.json", seed_id));

    if !finding_path.exists() {
        bail!(
            "No finding file at {}. Run `praxis fuzz` first to generate findings.",
            finding_path.display()
        );
    }

    let raw = std::fs::read_to_string(&finding_path)
        .with_context(|| format!("reading {}", finding_path.display()))?;

    let finding: serde_json::Value =
        serde_json::from_str(&raw).context("parsing finding JSON")?;

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
    // Phase 1 basic: delegate to `cargo test` with PRAXIS_FUZZ env var so that
    // invariant test harnesses switch to fuzz mode (many iterations).
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
        std::fs::write(
            &toml_path,
            include_str!("../default_praxis.toml"),
        )
        .with_context(|| format!("writing {}", toml_path.display()))?;
        println!("Created {}", toml_path.display());
    } else {
        println!("{} already exists, skipping.", toml_path.display());
    }

    println!("Created {}", findings_dir.display());
    println!("Praxis initialised. Edit praxis.toml to configure your program.");
    Ok(())
}
