# CLAUDE.md — Praxis Build Instructions

> **You are building Praxis: a Rust-native testing, fuzzing, and profiling framework for Solana programs.** This document is your contract. Read it before every session. Do not deviate from it without asking.

---

## 1. What Praxis is (in one paragraph)

Praxis is a Rust crate, a CLI, and a small set of proc macros. It collapses the fragmented Solana test toolchain (LiteSVM, Mollusk, Surfpool, solana-program-test, Anchor's built-in framework) into a **single backend-agnostic API** that works against Anchor, Pinocchio, and Steel programs. On top of that runtime layer it adds three things that don't currently exist as integrated tooling on Solana: **line-level CU profiling**, **Solana-aware property-based invariant fuzzing with adversarial account generation**, and a **pre-audit report generator** that produces auditor-ready artifacts. The wedge is bug-class economics: 85.5% of severe Solana audit findings and 53% of all on-chain losses come from access-control and business-logic bugs that runtime fuzzing can find but static analysis cannot.

## 2. What Praxis is NOT

- **Not a programming framework.** It does not change how programs are written. It is a harness around them.
- **Not a replacement for Anchor or Pinocchio.** Both are first-class targets.
- **Not a static analyser.** It complements sol-fuzz; fuzzing is execution-driven.
- **Not an audit replacement.** It produces pre-audit artifacts that reduce audit scope.
- **Not a custom SVM.** It wraps existing SVMs (LiteSVM, Mollusk, Surfpool) under a unified trait.

## 3. Locked tech stack — DO NOT debate or substitute

| Concern | Choice | Notes |
|---|---|---|
| Language | Rust, edition `2024` | MSRV `1.82` |
| Default SVM backend | `litesvm` | Fastest in-memory; default for fuzz loops |
| CU backend | `mollusk-svm` | For per-instruction CU isolation |
| Fork backend | Surfpool (RPC client) | Mainnet-fork integration tests only |
| Property testing | `proptest` | For shrinking machinery |
| IDL parsing (Anchor) | `anchor-lang-idl` | Anchor 0.30+ |
| IDL parsing (Codama/Shank) | `codama-rs`, `shank` | Pinocchio/Steel path |
| Async runtime | `tokio` (Surfpool only) | LiteSVM/Mollusk are sync |
| Parallelism | `rayon` | Data-parallel fuzz workers |
| CLI | `clap` v4 with derive | Subcommand structure |
| Config | `toml` + `serde` | praxis.toml |
| Logging | `tracing` + `tracing-subscriber` | Structured, JSON output supported |
| Flame graph | `inferno` | SVG output |
| License | Dual `Apache-2.0 OR MIT` | Standard Rust dual-license |

If a task seems to require a crate not on this list, **ask first** before adding it.

## 4. Coding standards

- **Rust edition `2024`**, MSRV pinned to `1.82` in workspace `Cargo.toml`.
- **`#![deny(unsafe_code)]`** at every crate root unless a crate has a documented exception (none expected in v0.1).
- **`#![warn(missing_docs)]`** on every public item once a module stabilises (defer for in-progress modules).
- **Error handling:** use `thiserror` for library crates, `anyhow` only in the CLI binary. Never `unwrap()` / `expect()` outside tests; use `?`.
- **Formatting:** `cargo fmt` clean. `cargo clippy --all-targets --all-features -- -D warnings` clean.
- **Imports:** group `std`, then external crates, then `crate::`. Use `use foo::{bar, baz}` not nested.
- **No `tokio` outside the Surfpool crate.** Async leaks complicate everything.
- **No `Box<dyn Error>`** in public APIs. Use a typed error enum per crate.
- **Public APIs are flat and macro-light.** Macros are convenience only — every macro must have an underlying function-call form that does the same thing.
- **Determinism:** every randomised path takes a `u64` seed. No global RNG. Ever.

## 5. Repository layout to create

Create this exact cargo workspace. Crates that are not yet implemented should be created as empty crates with a `lib.rs` containing only a `//! TODO(phase-N)` comment so the workspace builds from day one.

```
praxis/
├── Cargo.toml              # workspace root with [workspace] members
├── Cargo.lock
├── README.md
├── CLAUDE.md               # this file
├── LICENSE-APACHE
├── LICENSE-MIT
├── rust-toolchain.toml     # pin to 1.82
├── .github/workflows/ci.yml
├── praxis.toml             # self-test config (added in Phase 2)
├── crates/
│   ├── praxis/             # umbrella public crate
│   ├── praxis-core/        # core types, Svm trait, NormalIdl
│   ├── praxis-idl/         # IDL ingestion (Anchor, Codama, Shank)
│   ├── praxis-svm-litesvm/ # LiteSVM backend (Phase 1)
│   ├── praxis-svm-mollusk/ # Mollusk backend (Phase 2)
│   ├── praxis-svm-surfpool/# Surfpool backend (Phase 3)
│   ├── praxis-gen/         # adversarial generators + shrinker
│   ├── praxis-fuzz/        # invariant fuzzer engine
│   ├── praxis-profile/     # CU profiler (Phase 2)
│   ├── praxis-checks/      # FD/T-2022/AC check pack (Phase 2)
│   ├── praxis-report/      # markdown + JSON report (Phase 3)
│   ├── praxis-macros/      # proc macros: #[invariant_test], #[profile]
│   └── praxis-cli/         # binary
├── examples/
│   ├── escrow-anchor/      # reference Anchor program with planted bugs
│   ├── amm-pinocchio/      # CU-sensitive Pinocchio AMM
│   └── token-2022-hook/    # Transfer Hook program (Phase 2)
├── tests/
│   └── e2e/                # end-to-end tests across all crates
└── docs/
    ├── architecture.md
    ├── writing-invariants.md
    └── check-pack-reference.md
```

## 6. Architecture in one screen

Five horizontal layers. Each layer depends only on layers below it. The `Svm` trait at Layer 2 is the linchpin; keep it small.

```
+---------------------------------------------------------------+
|                      LAYER 5: CLI & REPORT                    |
|   praxis test | praxis fuzz | praxis profile | praxis report  |
+---------------------------------------------------------------+
|                  LAYER 4: HIGHER-ORDER ENGINES                |
|   Invariant Fuzzer | CU Profiler | Check Pack | Diff Engine   |
+---------------------------------------------------------------+
|                LAYER 3: ADVERSARIAL GENERATORS                |
|    Account Mutators | Tx Composer | Seed Strategy | Shrinker  |
+---------------------------------------------------------------+
|              LAYER 2: UNIFIED RUNTIME ABSTRACTION             |
|       trait Svm { execute, account, snapshot, restore }       |
+---------------------------------------------------------------+
|                    LAYER 1: SVM BACKENDS                      |
|   LiteSvmBackend | MolluskBackend | SurfpoolBackend (forking) |
+---------------------------------------------------------------+
|                LAYER 0: SCHEMA / IDL INGESTION                |
|   Anchor IDL parser | Codama parser | Shank annotation parser |
+---------------------------------------------------------------+
```

**Hard rule:** No layer ever imports from a layer above it. If you find yourself wanting to, the abstraction is wrong — stop and ask.

## 7. The Svm trait — the most important type in the codebase

This trait lives in `praxis-core`. Every backend implements it. The trait is small **on purpose**. Do not add methods without explicit approval — extending it forces work into every backend.

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

pub struct ExecResult {
    pub success: bool,
    pub cu_consumed: u64,
    pub logs: Vec<String>,
    pub return_data: Option<Vec<u8>>,
    pub error: Option<TransactionError>,
    pub mutated_accounts: Vec<Pubkey>,
}

pub struct SvmCapabilities {
    pub mainnet_fork: bool,
    pub cu_introspection: bool,
    pub cheatcodes: bool,
    pub parallel_safe: bool,
}
```

## 8. Build phases

You will build in **three phases**. Do not start Phase 2 work while Phase 1 has open items. Do not start Phase 3 work while Phase 2 has open items.

### Phase 1 — Core runtime + invariant fuzzer (target: 6 weeks)

**Goal:** a working fuzzer against Anchor programs on LiteSVM. If only this ships, Praxis is already useful.

**Definition of Done for Phase 1:**
- [ ] Workspace builds clean with `cargo build --workspace`
- [ ] `cargo test --workspace` passes
- [ ] CI workflow green on Linux + macOS
- [ ] `examples/escrow-anchor` builds and has at least 3 planted bugs
- [ ] `praxis test` runs invariants against `escrow-anchor` and detects all 3 planted bugs
- [ ] Each detected bug produces a deterministic seed and reproducer command
- [ ] Throughput: minimum 10,000 fuzz iterations / second on a reference 8-core machine, single LiteSVM-backed program

### Phase 2 — Profiler, check pack, second backend (target: 5 weeks)

**Goal:** CU profiler + check pack + Mollusk backend. Praxis now competes on multiple axes.

**Definition of Done for Phase 2:**
- [ ] Mollusk backend implements `Svm` trait, all `praxis test` examples pass against both backends
- [ ] `praxis profile` emits an SVG flame graph with at least function-level CU attribution
- [ ] `praxis profile diff <ref>` shows CU delta between two commits
- [ ] Check pack implements: AC-001, AC-002, CPI-001, FD-001, FD-002, FD-003, T22-001, T22-002, T22-003
- [ ] Pinocchio + Steel IDL ingestion via Shank works on `examples/amm-pinocchio`
- [ ] `examples/token-2022-hook` triggers T22-001 and T22-002 as expected

### Phase 3 — Reporting, mainnet fork, polish (target: 3 weeks)

**Goal:** auditor-grade output + Surfpool integration. Public v0.1 launch.

**Definition of Done for Phase 3:**
- [ ] `praxis report` emits both Markdown and JSON, JSON validates against published schema
- [ ] Surfpool backend works for mainnet-fork integration tests
- [ ] Docs site live (mdBook in `docs/`) with quickstart, writing-invariants guide, check-pack reference
- [ ] All three reference programs ship with passing CI
- [ ] Bug-bounty regression corpus: at least 7 of 10 historical exploits detected
- [ ] `praxis-cli` published to crates.io and installable via `cargo install praxis-cli`

## 9. Phase 1 — granular task list (start here)

Work through these in order. Open a TODO list at the start of each session and check items off.

### Week 1 — Workspace and CI

1. Create `Cargo.toml` workspace with all 12 crates listed (empty `lib.rs` with `TODO` comment).
2. Add `rust-toolchain.toml` pinning to `1.82`.
3. Add `LICENSE-APACHE` and `LICENSE-MIT` files.
4. Add `.gitignore` (standard Rust + `.praxis/` for runtime artifacts).
5. Add GitHub Actions CI: `fmt`, `clippy`, `test --workspace` on Linux and macOS.
6. In `praxis-core`:
   - Define `Svm` trait exactly as in section 7
   - Define `ExecResult`, `SvmCapabilities`, `SvmSnapshot` types
   - Define `NormalIdl`, `NormalInstruction`, `AccountMeta`, `AccountConstraint` per the spec
   - Add an in-memory `MockSvm` that returns canned results — used for unit-testing higher layers without a real backend
7. Verify `cargo build --workspace` and `cargo test --workspace` are green.

### Week 2 — LiteSVM backend

1. In `praxis-svm-litesvm`, add `litesvm` as dependency.
2. Implement `LiteSvmBackend` struct wrapping `LiteSVM`.
3. Implement all `Svm` trait methods. Snapshot/restore must be O(state-size) not O(slot-history) — clone the relevant account map.
4. Capability flags: `mainnet_fork=false`, `cu_introspection=true`, `cheatcodes=true`, `parallel_safe=true`.
5. Unit tests: deploy a tiny test program, execute a transaction, verify CU and logs.
6. Property test: snapshot → mutate → restore → state matches original.

### Week 3 — Anchor IDL ingestion

1. In `praxis-idl`, add `anchor-lang-idl` dependency.
2. Implement `parse_anchor_idl(path: &Path) -> Result<NormalIdl>`.
3. Map every Anchor account constraint to `AccountConstraint` enum variants.
4. Map PDA seed expressions to `PdaRule` (must support literal seeds, account-derived seeds, and bumps).
5. Snapshot tests: input several real Anchor IDLs (e.g. SPL Token, a sample escrow), assert output matches a checked-in snapshot.

### Week 4 — Generators

1. In `praxis-gen/src/account.rs`: implement `AccountSpawner` that, given an `AccountMeta`, generates a valid account that satisfies its constraints (correct owner, correct PDA, signer flag, etc.).
2. In `praxis-gen/src/mutation.rs`: implement these mutation strategies as enum variants:
   - `MissingSigner` (drop is_signer)
   - `WrongOwner` (replace owner with random program ID)
   - `WrongPdaSeeds` (substitute PDA derived from wrong seeds)
   - `FakeProgram` (replace CPI program ID)
   - `DuplicateAccount` (alias two account slots)
   - `UninitializedRead` (pass freshly created account)
3. Each mutation strategy is a pure function: `(NormalInstruction, AccountSet, Seed) -> AccountSet`.
4. In `praxis-gen/src/tx.rs`: implement `TxComposer` that turns `(NormalInstruction, AccountSet, Args)` into a `Transaction`.
5. In `praxis-gen/src/shrink.rs`: implement basic shrinking — for now, just instruction-list truncation. Full proptest integration in Week 5.

### Week 5 — Invariant fuzzer

1. In `praxis-fuzz`: define the `Ctx` type and the public fuzzing API (`spawn_signer`, `spawn_account`, `spawn_pda`, `invariant`, `fuzz_instructions`, `with_mutations`, `run`).
2. Implement the fuzz loop: snapshot → generate → execute → check invariants → restore on failure.
3. Wire `proptest` for shrinking failed transaction sequences.
4. Persist failures to `.praxis/findings/<id>.json` with seed, mutation set, transaction trace.
5. Implement `Ctx::replay(seed)` for deterministic reproduction.

### Week 6 — Macros, CLI bootstrap, escrow example

1. In `praxis-macros`: implement `#[invariant_test]` proc macro. It should expand to a function the test runner can discover via inventory or linkme.
2. In `praxis-cli`: implement `praxis test` and `praxis replay --seed <hex>`. Other subcommands stub out with "not implemented in Phase 1".
3. In `examples/escrow-anchor`: build a minimal Anchor escrow program. Plant 3 bugs:
   - Missing signer check on the `release` instruction's authority account
   - Owner check missing on the deserialised escrow account
   - Wrong-PDA-seed acceptance in `cancel`
4. Write Praxis invariants that catch all 3 bugs.
5. End-to-end test in `tests/e2e/escrow.rs`: run the fuzzer, assert all 3 findings produced.
6. **Mark Phase 1 complete only when the e2e test passes deterministically across 100 consecutive runs.**

## 10. Testing strategy

- **Unit tests** in every crate. `praxis-core`, `praxis-idl`, `praxis-gen` should target ≥80% line coverage.
- **Integration tests** under `tests/e2e/` exercise the full stack against the example programs.
- **Differential tests** (Phase 2+): the same invariant test executed against LiteSVM, Mollusk, and Surfpool must produce the same finding set (modulo backend-specific capabilities).
- **Bug-bounty regression corpus** (Phase 3): historical Solana exploits (Wormhole, Cashio, Mango, Solend, Drift, Loopscale) reproduced as test cases. Praxis must detect each on the affected commit. Lives in `tests/regression/`.
- **Determinism gate:** every test that uses randomness must run 100x in CI and produce identical results. Flaky tests are a release-blocker, never tolerated.

## 11. Adversarial mutation taxonomy (full list, for reference)

You'll implement these incrementally. Phase 1 covers the first 6.

| Strategy | Mutation | Targets bug class |
|---|---|---|
| `MissingSigner` | Drop `is_signer` flag | Signer-check bypass |
| `WrongOwner` | Replace owner with random program ID | Owner-check bypass |
| `WrongPdaSeeds` | Substitute PDA derived from wrong seeds | PDA spoofing |
| `FakeProgram` | Replace CPI target with attacker-controlled program | Arbitrary CPI |
| `DuplicateAccount` | Pass same account in two slots | Aliasing logic |
| `UninitializedRead` | Pass freshly created account where init expected | Init-check bypass |
| `LamportsDrain` | Pass account with rent-exempt minimum | Lamport accounting |
| `TokenMintMismatch` | Provide token account with wrong mint | Token validation |
| `HookExtraAccountInjection` | Inject malicious accounts via ExtraAccountMetaList | Token-2022 hook abuse |
| `StalenessSimulation` | Warp clock to invalidate oracle freshness | Stale-price exploit |
| `DiscriminatorCollision` | Send instruction with overlapping discriminator | Account type confusion |

## 12. Check pack (Phase 2 — full list)

| Check ID | What it asserts | Trigger |
|---|---|---|
| `AC-001` | Every authority parameter has signer constraint | Static + runtime |
| `AC-002` | Every deserialised account has explicit owner check | Runtime fuzz |
| `CPI-001` | All CPIs target whitelisted program IDs | Runtime fuzz |
| `FD-001` | Protocol invariants hold across N skipped slots | Runtime, slot warp |
| `FD-002` | Pyth/Switchboard last_update_slot ≤ N slots old | Runtime |
| `FD-003` | Pyth confidence interval rejects above threshold | Runtime |
| `LFM-001` | No single account written by >10% of traffic | Static analysis |
| `T22-001` | Transfer Hook does not CPI back into same mint | Runtime fuzz |
| `T22-002` | All ExtraAccountMetaList seeds validated | Runtime fuzz |
| `T22-003` | ZK proof inputs match expected ciphertexts | Runtime |

## 13. CLI surface (final)

```bash
praxis init                 # scaffold .praxis/ directory and praxis.toml
praxis test                 # run all #[invariant_test] functions (Phase 1)
praxis fuzz                 # long-running adversarial fuzzing (Phase 1)
praxis profile              # CU flame graph (Phase 2)
praxis profile diff <ref>   # CU delta vs baseline commit (Phase 2)
praxis check                # check pack only, fast (Phase 2)
praxis replay --seed <hex>  # reproduce known finding (Phase 1)
praxis report               # emit Markdown + JSON pre-audit report (Phase 3)
praxis ci                   # all-in-one for CI, exit code per severity (Phase 3)
```

## 14. Configuration file format

Each project has a `praxis.toml`:

```toml
[program]
name = "escrow"
path = "./target/deploy/escrow.so"
idl  = "./target/idl/escrow.json"

[backend]
default = "litesvm"
fuzz    = "litesvm"
profile = "mollusk"
forked  = "surfpool"

[fuzz]
iterations    = 50_000
seed          = 0xDEADBEEF
parallel      = 8
budget_secs   = 600
mutations     = ["MissingSigner", "WrongOwner", "WrongPdaSeeds"]

[checks]
enabled = ["AC-001", "AC-002", "CPI-001", "FD-001", "FD-002", "T22-001"]

[report]
output_dir = "./.praxis/reports"
formats    = ["markdown", "json"]
fail_on    = "high"
```





## 15. What NOT to do

- **Do not start Phase 2 work in Phase 1.** No premature CU profiling, no premature check pack scaffolding. Resist.
- **Do not add `unsafe` blocks.** If you think you need one, stop and ask.
- **Do not use `tokio` outside `praxis-svm-surfpool`.** Async leaks are how this becomes unmaintainable.
- **Do not extend the `Svm` trait without approval.** Every method added is work multiplied across N backends.
- **Do not produce findings without reproducers.** Every finding must include a deterministic seed and replay command. False positives are existential.
- **Do not over-macro.** Anchor V2 feedback was explicit: less macros. Functions first, macros only as ergonomic sugar.
- **Do not invent crate names not on the locked stack list.** Ask first.
- **Do not skip determinism tests.** Every randomised path needs a seed and a replay test.
- **Do not write integration tests that require network.** Surfpool tests are gated behind a feature flag and run only in dedicated CI jobs.

## 16. First session goals

When you start the first coding session, your goals in order are:

1. Run `git init` if not already done. Confirm `.gitignore` is correct.
2. Create the workspace `Cargo.toml` and all 12 empty crates (each with `lib.rs` containing `//! TODO(phase-1)` comments).
3. Add `rust-toolchain.toml`, both LICENSE files, README.md skeleton.
4. Add the GitHub Actions CI workflow.
5. Verify `cargo build --workspace` is green.
6. Commit with message: `chore: initial workspace scaffold`.

Stop there. Confirm with the human before moving to Week 1 task 6 (defining the `Svm` trait in `praxis-core`).

## 17. Key references (for context, do not paste verbatim into code)

- Sec3 Solana Security Ecosystem Review 2025 — bug taxonomy
- Hacken 2025 Yearly Security Report — loss attribution
- Anchor V2 RFP (`solana-foundation/anchor` discussion #3742) — community pain points
- Solana Foundation Stride program (April 2026) — long-term distribution channel
- Helius engineering blog: Pinocchio framework, Solana Program Security
- Zealynx Solana Smart Contract Audit Guide 2026 — Firedancer + Token-2022 threats
- `solana-foundation/solana-dev-skill` — March 2026 best practices

## 18. When to stop and ask

**Always ask the human before:**
- Adding a dependency not on the locked stack
- Extending the `Svm` trait
- Skipping a Phase's Definition-of-Done item
- Starting work in a later phase
- Designing a feature that isn't in this document
- Making a breaking change to a public API after it has been used by another crate

**Otherwise: proceed and ship.**

---

*End of CLAUDE.md. If this file is ambiguous on something you need to do, that's a bug — flag it and ask the human to amend the file rather than guess.*