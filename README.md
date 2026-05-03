# Praxis

**Rust-native testing, fuzzing, and pre-audit tooling for Solana programs.**

Praxis collapses the fragmented Solana test toolchain — LiteSVM, Mollusk, Surfpool, and Anchor's built-in framework — into a **single backend-agnostic API**. On top of that runtime layer it adds three capabilities that do not exist as integrated tooling on Solana today:

- **Invariant fuzzing** with Solana-aware adversarial account generation and proptest shrinking
- **Line-level CU profiling** with flame graphs and commit-to-commit diff (Phase 2)
- **Pre-audit report generation** producing auditor-ready Markdown and JSON artifacts (Phase 3)

The threat model is concrete: 85.5 % of severe Solana audit findings and 53 % of on-chain losses come from access-control and business-logic bugs that static analysis cannot find. Praxis finds them at fuzz time, before deployment.

---

## Contents

- [Status](#status)
- [Installation](#installation)
- [Quick start](#quick-start)
- [Configuration — praxis.toml](#configuration--praxistoml)
- [CLI reference](#cli-reference)
- [Writing invariant tests](#writing-invariant-tests)
- [Mutation strategies](#mutation-strategies)
- [Findings and replay](#findings-and-replay)
- [Crate overview](#crate-overview)
- [Architecture](#architecture)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License](#license)

---

## Status

| Phase | Goal | State |
|---|---|---|
| **Phase 1** | Core runtime + invariant fuzzer on LiteSVM | **In progress** |
| Phase 2 | CU profiler, check pack, Mollusk backend | Planned |
| Phase 3 | Reporting, mainnet fork (Surfpool), public release | Planned |

Phase 1 Definition of Done (tracked in `CLAUDE.md`):
- [x] Workspace builds clean — `cargo build --workspace`
- [x] LiteSVM backend with snapshot/restore
- [x] Anchor IDL ingestion → `NormalIdl`
- [x] Adversarial generators (6 Phase 1 mutation strategies)
- [x] Invariant fuzzer engine with proptest shrinking
- [x] `#[invariant_test]` proc macro
- [x] `praxis test`, `praxis fuzz`, `praxis replay`, `praxis init` CLI
- [x] `examples/escrow-anchor` with 3 planted bugs
- [ ] End-to-end test detecting all 3 escrow bugs deterministically
- [ ] 10,000 fuzz iterations/second on 8-core reference machine

---

## Installation

**Requirements:** Rust 1.82+ (toolchain is pinned via `rust-toolchain.toml`).

```bash
# Install from source (until crates.io publish in Phase 3)
git clone https://github.com/E-Okelloh/praxis.git
cd praxis
cargo install --path crates/praxis-cli
```

Verify:

```bash
praxis --version
```

---

## Quick start

### 1. Initialise a project

Inside your Solana workspace:

```bash
praxis init
```

This creates `.praxis/findings/` and a `praxis.toml` config file pre-filled with sensible defaults.

### 2. Edit praxis.toml

```toml
[program]
name = "my_program"
path = "./target/deploy/my_program.so"
idl  = "./target/idl/my_program.json"   # Anchor IDL

[fuzz]
iterations = 50_000
seed       = 0xDEADBEEF
parallel   = 8
mutations  = ["MissingSigner", "WrongOwner", "WrongPdaSeeds"]
```

### 3. Write an invariant test

Add `praxis-fuzz` and `praxis-macros` to your `[dev-dependencies]`, then:

```rust
use praxis_fuzz::{Ctx, FuzzError};
use praxis_macros::invariant_test;
use praxis_svm_litesvm::LiteSvmBackend;
use solana_sdk::pubkey::Pubkey;

const PROGRAM_ID: Pubkey = solana_sdk::pubkey!("Esc1...");

#[invariant_test]
fn escrow_authority_cannot_be_spoofed() {
    let svm = Box::new(LiteSvmBackend::new());
    let ctx = Ctx::new(svm, PROGRAM_ID)
        .with_seed(0xDEADBEEF)
        .with_iterations(10_000);

    ctx.invariant("authority_holds_lamports", |before, after, _result| {
        // Lamports must never leave the vault unless the real authority signed.
        let vault_before = before.lamports(&VAULT_PDA);
        let vault_after  = after.lamports(&VAULT_PDA);
        if vault_after < vault_before {
            return Err(FuzzError::invariant_violated(
                "vault drained without authority signature",
            ));
        }
        Ok(())
    });

    ctx.run().expect("fuzzer found a violation");
}
```

### 4. Run

```bash
# Run all invariant tests once (fast, for CI)
praxis test

# Long-running adversarial fuzz loop
praxis fuzz

# Fuzz a specific test binary with extra cargo flags
praxis fuzz -- --test escrow_tests
```

---

## Configuration — praxis.toml

Full schema:

```toml
[program]
name = "escrow"
path = "./target/deploy/escrow.so"
idl  = "./target/idl/escrow.json"

[backend]
default = "litesvm"    # used by `praxis test`
fuzz    = "litesvm"    # used by `praxis fuzz`
profile = "mollusk"    # Phase 2
forked  = "surfpool"   # Phase 3

[fuzz]
iterations    = 50_000
seed          = 0xDEADBEEF
parallel      = 8
budget_secs   = 600
mutations     = ["MissingSigner", "WrongOwner", "WrongPdaSeeds",
                 "FakeProgram", "DuplicateAccount", "UninitializedRead"]

[checks]
# Phase 2 — leave empty for Phase 1
enabled = []

[report]
output_dir = "./.praxis/reports"
formats    = ["markdown", "json"]
fail_on    = "high"
```

---

## CLI reference

```
USAGE:
    praxis <SUBCOMMAND>

SUBCOMMANDS:
    init      Scaffold .praxis/ directory and praxis.toml
    test      Run all #[invariant_test] functions in the workspace
    fuzz      Long-running adversarial fuzzing (sets PRAXIS_FUZZ=1)
    replay    Reproduce a known finding: praxis replay --seed <ID>
    profile   CU flame graph  [Phase 2]
    check     Fast check pack [Phase 2]
    report    Markdown + JSON pre-audit report [Phase 3]
    ci        All-in-one CI run with exit code per severity [Phase 3]
```

### praxis init

```bash
praxis init [--path <dir>]
```

Creates `.praxis/findings/` and writes a default `praxis.toml` if one does not exist.

### praxis test

```bash
praxis test [--path <dir>] [-- <cargo test args>]
```

Delegates to `cargo test`. All functions annotated with `#[invariant_test]` expand to `#[test]` so the standard test runner discovers them. Use `-- --test-threads 1` for deterministic serial execution.

### praxis fuzz

```bash
praxis fuzz [-- <cargo test args>]
```

Sets `PRAXIS_FUZZ=1`, then delegates to `cargo test`. Invariant test harnesses detect this env var and switch to high-iteration fuzz mode. Findings are written to `.praxis/findings/<ID>.json`.

### praxis replay

```bash
praxis replay --seed <finding-id>
```

Reads `.praxis/findings/<finding-id>.json`, pretty-prints the finding, and outputs the exact command to reproduce it. Finding IDs are emitted by the fuzzer and have the form `<16-hex-seed>-<MutationName>`.

---

## Writing invariant tests

### The `#[invariant_test]` macro

Annotate any no-argument `fn` returning `()`:

```rust
#[praxis_macros::invariant_test]
fn my_invariant() {
    // build Ctx, register invariants, call ctx.run()
}
```

The macro expands to `#[test]`, so it runs with `cargo test` and with `praxis test`.

### Building a Ctx

```rust
use praxis_fuzz::Ctx;
use praxis_gen::MutationStrategy;
use praxis_svm_litesvm::LiteSvmBackend;

let ctx = Ctx::new(Box::new(LiteSvmBackend::new()), program_id)
    .with_seed(0xCAFEBABE)           // deterministic RNG seed
    .with_iterations(50_000)          // iterations per run
    .with_mutations(vec![
        MutationStrategy::MissingSigner,
        MutationStrategy::WrongOwner,
        MutationStrategy::WrongPdaSeeds,
    ])
    .with_findings_dir(".praxis/findings");
```

### Registering invariants

```rust
ctx.invariant("no_lamport_drain", |before, after, result| {
    // `before` and `after` are SvmSnapshot — call .lamports(&pubkey) etc.
    // `result` is the ExecResult from the fuzzed transaction.
    // Return Ok(()) to pass, Err(FuzzError::...) to flag a violation.
    Ok(())
});
```

Multiple invariants can be registered; all are checked after each iteration.

### Spawning accounts

```rust
// Funded signer keypair loaded into the SVM:
let authority = ctx.spawn_signer(seed);

// Generic account (writable, no signer):
let vault = ctx.spawn_account(seed, owner_pubkey, lamports, data_len);

// PDA derived from seeds:
let (pda, bump) = ctx.spawn_pda(seed, &[b"escrow", authority.pubkey().as_ref()]);
```

---

## Mutation strategies

Phase 1 ships 6 strategies. Additional strategies land in Phase 2.

| Strategy | Mutation applied | Bug class targeted |
|---|---|---|
| `MissingSigner` | Drops `is_signer` flag from an account | Signer-check bypass |
| `WrongOwner` | Replaces account owner with a random program ID | Owner-check bypass |
| `WrongPdaSeeds` | Substitutes a PDA derived from wrong seeds | PDA spoofing |
| `FakeProgram` | Replaces a CPI target with an attacker-controlled program | Arbitrary CPI |
| `DuplicateAccount` | Aliases two account slots to the same pubkey | Account aliasing |
| `UninitializedRead` | Passes a freshly created (zeroed) account | Init-check bypass |

Phase 2 adds: `LamportsDrain`, `TokenMintMismatch`, `HookExtraAccountInjection`, `StalenessSimulation`, `DiscriminatorCollision`.

---

## Findings and replay

Every violation the fuzzer detects is serialised to `.praxis/findings/<ID>.json`:

```json
{
  "id": "deadbeef00000001-MissingSigner",
  "seed": 3735928559,
  "mutation": "MissingSigner",
  "invariant": "authority_holds_lamports",
  "instruction": "release",
  "tx_trace": [...],
  "replay_cmd": "praxis replay --seed deadbeef00000001-MissingSigner"
}
```

To reproduce:

```bash
praxis replay --seed deadbeef00000001-MissingSigner
```

Findings are **always deterministic**: the same seed and mutation applied to the same program always produces the same trace. This is a hard invariant of Praxis — findings without reproducers are never emitted.

---

## Crate overview

```
praxis/
├── crates/
│   ├── praxis               # umbrella re-export crate
│   ├── praxis-core          # Svm trait, ExecResult, NormalIdl, MockSvm
│   ├── praxis-idl           # Anchor IDL → NormalIdl parser
│   ├── praxis-svm-litesvm   # LiteSVM backend (Phase 1)
│   ├── praxis-svm-mollusk   # Mollusk backend (Phase 2)
│   ├── praxis-svm-surfpool  # Surfpool mainnet-fork backend (Phase 3)
│   ├── praxis-gen           # Adversarial generators + mutations + shrinker
│   ├── praxis-fuzz          # Invariant fuzzer engine (Ctx, Finding, engine)
│   ├── praxis-profile       # CU profiler + flame graph (Phase 2)
│   ├── praxis-checks        # AC / FD / T22 check pack (Phase 2)
│   ├── praxis-report        # Markdown + JSON report emitter (Phase 3)
│   ├── praxis-macros        # #[invariant_test], #[profile] proc macros
│   └── praxis-cli           # `praxis` binary
├── examples/
│   ├── escrow-anchor        # Anchor escrow with 3 planted bugs (Phase 1 e2e target)
│   ├── amm-pinocchio        # CU-sensitive Pinocchio AMM (Phase 2)
│   └── token-2022-hook      # Transfer Hook program (Phase 2)
└── tests/
    └── e2e/                 # End-to-end tests across the full stack
```

### Key types

| Type | Crate | Role |
|---|---|---|
| `trait Svm` | `praxis-core` | Backend abstraction — every SVM implements this |
| `ExecResult` | `praxis-core` | Transaction result: success, CU, logs, error |
| `NormalIdl` | `praxis-core` | Backend-agnostic program schema |
| `Ctx` | `praxis-fuzz` | User-facing fuzzing context |
| `Finding` | `praxis-fuzz` | Serialised violation with seed and trace |
| `MutationStrategy` | `praxis-gen` | Enum of adversarial account mutations |
| `AccountSet` | `praxis-gen` | Ordered account slots for one instruction |
| `TxComposer` | `praxis-gen` | Builds a `Transaction` from instruction + accounts |

---

## Architecture

Five horizontal layers. Each layer depends only on layers below it.

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

The `Svm` trait at Layer 2 is the linchpin. It is intentionally small — every method added multiplies work across all backends:

```rust
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

---

## Roadmap

### Phase 2 — Profiler, check pack, Mollusk backend

- `praxis profile` — SVG flame graph with function-level CU attribution
- `praxis profile diff <ref>` — CU delta between two commits
- Mollusk backend implementing the `Svm` trait
- Check pack: AC-001, AC-002, CPI-001, FD-001/2/3, T22-001/2/3
- Pinocchio + Steel IDL ingestion via Shank
- `examples/amm-pinocchio` and `examples/token-2022-hook`

### Phase 3 — Reporting, mainnet fork, public release

- `praxis report` — Markdown and JSON pre-audit reports
- Surfpool backend for mainnet-fork integration tests
- Bug-bounty regression corpus (Wormhole, Cashio, Mango, Solend, Drift, Loopscale)
- `cargo install praxis-cli` from crates.io
- mdBook documentation site

---

## Contributing

Praxis is in active Phase 1 development. Contributions are welcome once the Phase 1 Definition of Done is complete.

**Before contributing:**
- Read `CLAUDE.md` — it is the authoritative build contract.
- Every PR must pass `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --workspace`.
- No `unsafe` code. No `unwrap()` outside tests.
- Every randomised code path must accept a `u64` seed and be deterministically reproducible.

---

## License

Licensed under either of:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT license](LICENSE-MIT)

at your option.
