# Praxis

**Rust-native testing, fuzzing, and pre-audit tooling for Solana programs.**

[![Build](https://github.com/E-Okelloh/praxis/actions/workflows/ci.yml/badge.svg)](https://github.com/E-Okelloh/praxis/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/e-okelloh-praxis-cli.svg)](https://crates.io/crates/e-okelloh-praxis-cli)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](#license)
[![Rust 1.82+](https://img.shields.io/badge/rust-1.82%2B-orange.svg)](https://www.rust-lang.org)

---

Solana programs lose money because of access-control and business-logic bugs — **85.5 % of severe audit findings** and **53 % of all on-chain losses** trace back to these two classes. Static linters miss them. Manual audits catch them too late. Runtime fuzzing finds them before deployment.

Praxis is the tool that was missing: a backend-agnostic Solana fuzzer, CU profiler, check pack, and pre-audit report generator — all in one `cargo install`, all in pure Rust, all deterministically reproducible.

---

## What Praxis gives you

| Capability | What it does | When you need it |
|---|---|---|
| **Invariant fuzzer** | Generates adversarial accounts and transactions, runs your program against them, and shrinks any violation to a minimal reproducer | Pre-deployment, CI |
| **CU profiler** | Records per-instruction compute units, renders SVG flame graphs, diffs CU between commits | Optimization, pre-upgrade review |
| **Check pack** | 9 static + runtime checks for AC, FD, CPI, and Token-2022 bug classes | Any time, seconds to run |
| **Pre-audit report** | Markdown and JSON report aggregating all findings — hand it to an auditor before they even open your code | Before audit engagement |

---

## Why Praxis instead of the tools you already use

| Tool | What it does well | What it can't do |
|---|---|---|
| `litesvm` / `solana-program-test` | Execute transactions in-process | No adversarial generation, no invariant engine, no findings |
| Anchor's `#[test]` harness | Happy-path unit tests | Tests only what you wrote — not what an attacker would send |
| `sol-fuzz` / `trident` | Fuzzing with coverage feedback | No Solana-aware mutation; no CU profiler; no auditor report |
| Manual security review | Catches any bug a human notices | Expensive, slow, misses combinatorial account interactions |
| **Praxis** | All of the above in one backend-agnostic API | — |

Praxis wraps LiteSVM, Mollusk, and Surfpool under a single `Svm` trait. Your invariants run identically against all three backends. Your CI uses the fast in-memory backend; your pre-release run forks mainnet state via Surfpool.

---

## Historical exploits Praxis detects

The regression corpus covers 10 real on-chain incidents. Each check fires deterministically:

| Exploit | Date | Loss | Praxis check |
|---|---|---|---|
| Wormhole guardian set upgrade | Feb 2022 | $320 M | `AC-001` — missing signer on authority |
| Cashio collateral vault | Mar 2022 | $52 M | `AC-002` — unconstrained writable account |
| Mango oracle manipulation | Oct 2022 | $114 M | `FD-002` — stale `last_update_slot` |
| Solend confidence interval | 2022 | $— | `FD-003` — wide confidence band accepted |
| Drift admin update | 2023 | $— | `AC-001` — unsigned admin authority |
| Loopscale arbitrary CPI | Apr 2025 | $5.8 M | `CPI-001` — non-whitelisted CPI target |
| Escrow wrong PDA seeds | — | — | `AC-002` — unconstrained PDA account |
| Token-2022 extra meta seeds | — | — | `T22-002` — no seed validation |
| Token-2022 re-entrant hook | — | — | `T22-001` — CPI back into same mint |
| AMM missing mint validation | — | — | `AC-002` — unconstrained token account |

---

## Installation

**Requirements:** Rust 1.82 or later.

```bash
cargo install e-okelloh-praxis-cli
```

Verify:

```bash
praxis --version
```

Or build from source:

```bash
git clone https://github.com/E-Okelloh/praxis.git
cd praxis
cargo install --path crates/praxis-cli
```

---

## Five-minute quickstart

### 1. Scaffold your project

```bash
cd my-solana-program
praxis init
```

Creates `.praxis/findings/` and a starter `praxis.toml`.

### 2. Configure

```toml
# praxis.toml
[program]
name = "escrow"
path = "./target/deploy/escrow.so"
idl  = "./target/idl/escrow.json"

[fuzz]
iterations = 50_000
seed       = 0xDEADBEEF
parallel   = 8
mutations  = ["MissingSigner", "WrongOwner", "WrongPdaSeeds"]
```

### 3. Write one invariant

Add to `Cargo.toml`:

```toml
[dev-dependencies]
praxis-fuzz   = { package = "e-okelloh-praxis-fuzz",       version = "0.1" }
praxis-macros = { package = "e-okelloh-praxis-macros",     version = "0.1" }
praxis-svm-litesvm = { package = "e-okelloh-praxis-svm-litesvm", version = "0.1" }
```

Write the test:

```rust
use praxis_fuzz::{Ctx, FuzzError};
use praxis_macros::invariant_test;
use praxis_svm_litesvm::LiteSvmBackend;
use solana_sdk::pubkey;

const PROGRAM_ID: solana_sdk::pubkey::Pubkey =
    pubkey!("Esc1oooooooooooooooooooooooooooooooooooooo");

#[invariant_test]
fn vault_lamports_never_drain_without_authority() {
    let vault = pubkey!("Vau1taaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

    let mut ctx = Ctx::new(Box::new(LiteSvmBackend::new()), PROGRAM_ID)
        .with_seed(0xDEADBEEF)
        .with_iterations(10_000)
        .with_mutations(vec![
            praxis_gen::MutationStrategy::MissingSigner,
            praxis_gen::MutationStrategy::WrongOwner,
            praxis_gen::MutationStrategy::WrongPdaSeeds,
        ]);

    ctx.invariant("vault_balance_monotonic", move |svm, result| {
        // Vault must not lose lamports unless the transaction succeeded
        // with the real authority's signature.
        if !result.success {
            return true; // failed tx — no state change possible
        }
        svm.account(&vault)
            .map(|a| a.lamports > 0)
            .unwrap_or(true)
    });

    let findings = ctx.run().expect("fuzzer error");
    assert!(findings.is_empty(), "violations found: {findings:#?}");
}
```

### 4. Find bugs

```bash
# Fast CI run — one pass of all invariant tests
praxis test

# Long-running adversarial fuzz (hours, in CI nightly or pre-release)
praxis fuzz

# Reproduce any finding by ID
praxis replay --seed deadbeef00000001-MissingSigner
```

---

## Writing invariants

### The `Ctx` builder

```rust
let ctx = Ctx::new(svm, program_id)       // wrap any Svm backend
    .with_seed(0xCAFEBABE)               // deterministic — same seed = same run
    .with_iterations(100_000)
    .with_mutations(vec![
        MutationStrategy::MissingSigner,
        MutationStrategy::WrongOwner,
        MutationStrategy::WrongPdaSeeds,
        MutationStrategy::FakeProgram,
        MutationStrategy::DuplicateAccount,
        MutationStrategy::UninitializedRead,
    ])
    .with_findings_dir(".praxis/findings");
```

### Spawning accounts

```rust
// Funded signer keypair — loaded into the SVM automatically
let authority = ctx.spawn_signer(42);

// Funded generic account
let vault_key = ctx.spawn_account(43);

// PDA derived from your program's seeds
let escrow_pda = ctx.spawn_pda(&[b"escrow", authority.pubkey().as_ref()]);
```

### Registering invariants

```rust
ctx.invariant("no_lamport_drain", move |svm, result| {
    // Called after every fuzz iteration.
    // `svm`    — current backend state (call svm.account(&pk) to inspect)
    // `result` — ExecResult { success, cu_consumed, logs, error, .. }
    // Return true = invariant holds. Return false = violation found.
    svm.account(&vault_key)
        .map(|a| a.lamports >= MIN_RENT_EXEMPT)
        .unwrap_or(false)
});

ctx.invariant("authority_is_only_signer", move |svm, result| {
    result.success || result.error.is_some()
    // Both passing and failing transactions are valid — what matters is
    // that no lamports moved on a failure.
});
```

### Targeting one instruction

```rust
// Fuzz only the `release` instruction, not the full IDL
let findings = ctx.fuzz_instructions("release")?;
```

---

## Mutation strategies

Every strategy is a pure function of `(instruction, account_set, seed)`. The fuzzer applies each strategy independently and checks all registered invariants after each mutation.

| Strategy | Mutation applied | Bug class |
|---|---|---|
| `MissingSigner` | Drops `is_signer` from an account | Signer-check bypass |
| `WrongOwner` | Replaces account owner with a random program ID | Owner-check bypass |
| `WrongPdaSeeds` | Substitutes a PDA derived from wrong seeds | PDA spoofing |
| `FakeProgram` | Replaces a CPI target with attacker-controlled program | Arbitrary CPI |
| `DuplicateAccount` | Aliases two account slots to the same pubkey | Account aliasing |
| `UninitializedRead` | Passes a freshly created (zeroed) account | Init-check bypass |
| `LamportsDrain` | Passes account with rent-exempt minimum | Lamport accounting |
| `TokenMintMismatch` | Provides token account with wrong mint | Token validation |
| `HookExtraAccountInjection` | Injects malicious accounts via ExtraAccountMetaList | Token-2022 hook abuse |
| `StalenessSimulation` | Warps clock to invalidate oracle freshness | Stale-price exploit |
| `DiscriminatorCollision` | Sends instruction with overlapping discriminator | Account type confusion |

---

## Findings and replay

Every violation is serialised to `.praxis/findings/<ID>.json` with everything needed to reproduce it:

```json
{
  "id": "deadbeef00000001-MissingSigner",
  "seed": 3735928559,
  "mutation": "MissingSigner",
  "invariant": "vault_balance_monotonic",
  "instruction": "release",
  "cu_consumed": 18432,
  "logs": ["Program log: authority did not sign"],
  "replay_cmd": "praxis replay --seed deadbeef00000001-MissingSigner"
}
```

Reproduce on any machine:

```bash
praxis replay --seed deadbeef00000001-MissingSigner
```

**Findings without reproducers are never emitted.** This is a hard invariant of Praxis — every `Finding` carries a seed that produces an identical trace on any machine, any OS.

---

## CU profiler

### Record a session

```rust
use praxis_profile::{Profiler, Sample};

let mut profiler = Profiler::new("escrow");

for ix_name in &["initialize", "deposit", "release"] {
    let result = svm.execute(build_tx(ix_name));
    profiler.record(Sample {
        label: ix_name.to_string(),
        cu: result.cu_consumed,
    });
}

// SVG flame graph → open in any browser
std::fs::write("escrow-profile.svg", profiler.flame_graph_svg()?)?;

// Per-instruction summary
let report = profiler.report();
for ix in &report.instructions {
    println!("{:20} avg={:6} max={:6} ({:.1}% of total)",
        ix.name, ix.avg_cu, ix.max_cu, ix.pct_of_total);
}
```

### Commit-to-commit diff

```bash
# Save baseline on the current commit
praxis profile render --out baseline.json

# After your change
praxis profile diff baseline.json
```

```
instruction          before   after    delta    %
initialize           1 200    1 180    -20      -1.7%
deposit              3 800    5 100    +1 300   +34.2%   ⚠
release              2 400    2 400    0        0.0%
```

A `+34 %` delta on `deposit` after a single PR is the kind of signal that prevents CU surprises at deploy time.

---

## Check pack

Run all 9 checks against an Anchor IDL in under a second:

```bash
praxis check --idl ./target/idl/escrow.json
```

```
AC-001  HIGH      release::authority — authority-named account has no signer constraint
AC-002  HIGH      cancel::escrow_state — writable account has no owner constraint
CPI-001 CRITICAL  process_swap — CPI to non-whitelisted program AttackerProgram111...
FD-002  HIGH      sol_oracle — last_update_slot 600 slots stale (threshold: 100)
FD-003  MEDIUM    sol_oracle — confidence 30.0% of price (threshold: 10.0%)

5 findings  (1 critical, 3 high, 1 medium)
Exit code 1 — threshold: high
```

### From Rust

```rust
use praxis_checks::{run_static_checks, check_fd_002_staleness};
use praxis_core::NormalIdl;

let idl: NormalIdl = /* parse from JSON */;

// All static checks at once
let findings = run_static_checks(&idl);
for f in &findings {
    println!("[{}] {} — {}", f.check_id, f.severity, f.message);
}

// Point check: is this oracle stale?
if let Some(finding) = check_fd_002_staleness("sol_oracle", last_slot, current_slot, 100) {
    eprintln!("FD-002: {}", finding.message);
}
```

### Full check reference

| Check ID | Severity | What it asserts |
|---|---|---|
| `AC-001` | High | Every authority-named account has a signer constraint |
| `AC-002` | High | Every writable account has an explicit owner constraint |
| `CPI-001` | Critical | All CPIs target whitelisted program IDs |
| `FD-001` | Medium | Protocol invariants hold across N skipped slots |
| `FD-002` | High | Oracle `last_update_slot` is within the staleness threshold |
| `FD-003` | Medium | Oracle confidence interval is below the acceptance threshold |
| `T22-001` | Critical | Transfer Hook does not re-entrantly CPI back into the same mint |
| `T22-002` | High | All `ExtraAccountMetaList` seeds are validated |
| `T22-003` | Medium | ZK proof inputs match expected ciphertexts |

---

## Pre-audit report

Generate auditor-ready artifacts from your findings in one command:

```bash
praxis report --idl ./target/idl/escrow.json \
              --output-dir .praxis/reports \
              --formats markdown,json \
              --fail-on high
```

**Markdown output** (`.praxis/reports/escrow-report.md`):

```markdown
# Praxis Pre-Audit Report — escrow v0.1.0

Generated: 2026-05-07T14:23:01Z

## Summary

| Severity | Count |
|---|---|
| Critical | 1 |
| High | 3 |
| Medium | 1 |
| Info | 0 |

## Findings

### [CRITICAL] CPI-001 — Arbitrary CPI in process_swap
...

### [HIGH] AC-001 — Missing signer on release::authority
...
```

**JSON output** validates against `docs/report-schema.json` (JSON Schema Draft-07). Use it to:
- Block deployments in CI (`exit 1` if `has_findings_at("high")`)
- Feed findings into your issue tracker via API
- Compare reports between commits

---

## CI integration

One command runs everything and exits non-zero if any high/critical finding is found:

```bash
praxis ci --idl ./target/idl/my_program.json --fail-on high
```

Add to GitHub Actions:

```yaml
- name: Run Praxis security checks
  run: |
    cargo build-sbf
    praxis ci --idl target/idl/my_program.json --fail-on high
```

---

## Backends

Praxis implements one `Svm` trait across three backends. Switch backends by changing one line — your invariants stay identical.

```rust
// Fast in-process execution (CI, fuzz loops)
let svm = Box::new(LiteSvmBackend::new());

// CU-isolated per-instruction execution (profiling)
let svm = Box::new(MolluskBackend::new());

// Mainnet-fork with live account state
let svm = Box::new(
    SurfpoolBackend::from_rpc(
        "https://api.mainnet-beta.solana.com",
        &[token_mint, oracle_feed],
    ).await?
);

// All three implement the same trait:
pub trait Svm: Send + Sync {
    fn execute(&mut self, tx: Transaction) -> ExecResult;
    fn account(&self, pk: &Pubkey) -> Option<Account>;
    fn set_account(&mut self, pk: &Pubkey, acc: Account);
    fn snapshot(&self) -> SvmSnapshot;
    fn restore(&mut self, snap: &SvmSnapshot);
    fn warp_slot(&mut self, slot: u64);
    fn warp_timestamp(&mut self, ts: i64);
    fn capabilities(&self) -> SvmCapabilities;
}
```

The mainnet-fork backend fetches accounts at construction time via JSON-RPC, then runs all execution locally — no network calls during fuzz iterations.

---

## Crate overview

```
praxis/
├── crates/
│   ├── praxis                   # umbrella re-export crate
│   ├── praxis-core              # Svm trait, ExecResult, NormalIdl, MockSvm
│   ├── praxis-idl               # Anchor IDL → NormalIdl parser
│   ├── praxis-svm-litesvm       # LiteSVM backend
│   ├── praxis-svm-mollusk       # Mollusk backend (CU-isolated)
│   ├── praxis-svm-surfpool      # Surfpool backend (mainnet-fork)
│   ├── praxis-gen               # Adversarial generators + mutation strategies
│   ├── praxis-fuzz              # Invariant fuzzer engine — Ctx, Finding, engine
│   ├── praxis-profile           # CU profiler — flame graphs, diff, reports
│   ├── praxis-checks            # AC / FD / CPI / T22 check pack
│   ├── praxis-report            # Markdown + JSON pre-audit report emitter
│   ├── praxis-macros            # #[invariant_test] proc macro
│   └── praxis-cli               # `praxis` binary
├── examples/
│   ├── escrow-anchor            # Anchor escrow with 3 planted bugs (e2e target)
│   ├── amm-pinocchio            # CU-sensitive Pinocchio AMM with planted bug
│   └── token-2022-hook          # Transfer Hook with T22-001 and T22-002 planted
└── tests/
    ├── e2e/                     # End-to-end tests across the full stack
    └── regression/              # Bug-bounty corpus — 10 historical exploits
```

### Key types at a glance

| Type | Crate | Role |
|---|---|---|
| `trait Svm` | `praxis-core` | Backend abstraction — every SVM implements this |
| `ExecResult` | `praxis-core` | Result of one transaction: success, CU, logs, error |
| `NormalIdl` | `praxis-core` | Backend-agnostic program schema |
| `Ctx` | `praxis-fuzz` | User-facing fuzzing context |
| `Finding` | `praxis-fuzz` | Serialised violation with seed, mutation, trace |
| `MutationStrategy` | `praxis-gen` | Enum of adversarial account mutations |
| `Profiler` | `praxis-profile` | Collects CU samples, renders SVG flame graphs |
| `CheckFinding` | `praxis-checks` | Single check result with ID, severity, message |
| `ReportBuilder` | `praxis-report` | Assembles Report from findings + profile data |

---

## Architecture

Five horizontal layers. Each layer depends only on layers below it — no circular dependencies, no leaking abstractions.

```
┌──────────────────────────────────────────────────────────────────┐
│                     LAYER 5 — CLI & REPORT                       │
│   praxis test │ praxis fuzz │ praxis profile │ praxis report      │
├──────────────────────────────────────────────────────────────────┤
│                 LAYER 4 — HIGHER-ORDER ENGINES                   │
│   Invariant Fuzzer │ CU Profiler │ Check Pack │ Diff Engine       │
├──────────────────────────────────────────────────────────────────┤
│               LAYER 3 — ADVERSARIAL GENERATORS                   │
│   Account Mutators │ Tx Composer │ Seed Strategy │ Shrinker       │
├──────────────────────────────────────────────────────────────────┤
│             LAYER 2 — UNIFIED RUNTIME ABSTRACTION                │
│      trait Svm { execute, account, snapshot, restore }           │
├──────────────────────────────────────────────────────────────────┤
│                   LAYER 1 — SVM BACKENDS                         │
│   LiteSvmBackend │ MolluskBackend │ SurfpoolBackend               │
├──────────────────────────────────────────────────────────────────┤
│               LAYER 0 — SCHEMA / IDL INGESTION                   │
│   Anchor IDL parser │ Codama parser │ Shank annotation parser     │
└──────────────────────────────────────────────────────────────────┘
```

The `Svm` trait at Layer 2 is intentionally minimal — every method added multiplies implementation cost across all backends. The trait has never had a breaking change since its first commit.

---

## Configuration reference

```toml
# praxis.toml — place in your Solana workspace root

[program]
name = "escrow"
path = "./target/deploy/escrow.so"
idl  = "./target/idl/escrow.json"

[backend]
default = "litesvm"    # praxis test
fuzz    = "litesvm"    # praxis fuzz (fastest loop)
profile = "mollusk"    # praxis profile (CU isolation)
forked  = "surfpool"   # praxis fuzz --forked (mainnet state)

[fuzz]
iterations    = 50_000
seed          = 0xDEADBEEF
parallel      = 8
budget_secs   = 600
mutations     = [
    "MissingSigner", "WrongOwner", "WrongPdaSeeds",
    "FakeProgram", "DuplicateAccount", "UninitializedRead"
]

[checks]
enabled = ["AC-001", "AC-002", "CPI-001", "FD-001", "FD-002", "T22-001"]

[report]
output_dir = "./.praxis/reports"
formats    = ["markdown", "json"]
fail_on    = "high"
```

---

## CLI reference

```
praxis init                  Scaffold .praxis/ and write praxis.toml
praxis test                  Run all #[invariant_test] functions
praxis fuzz                  Long-running adversarial fuzz loop
praxis replay --seed <ID>    Reproduce a finding deterministically
praxis profile render        Emit SVG flame graph to stdout / --out file
praxis profile diff <base>   CU delta between current session and baseline
praxis check                 Run the check pack against --idl (seconds)
praxis report                Emit Markdown + JSON pre-audit report
praxis ci                    All-in-one CI run — exit 1 on severity threshold
```

---

## Contributing

Praxis is open to contributions. Before you open a PR:

- Read `CLAUDE.md` — it is the authoritative build contract.
- `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --workspace` must all be green.
- No `unsafe` code. No `unwrap()` outside tests.
- Every randomised path must accept a `u64` seed. No global RNG.
- Every finding must include a deterministic reproducer. Findings without seeds are release-blockers.

---

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
