# praxis

**Rust-native testing, fuzzing, and pre-audit tooling for Solana programs.**

[![Crates.io](https://img.shields.io/crates/v/e-okelloh-praxis.svg)](https://crates.io/crates/e-okelloh-praxis)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](#license)

This is the **umbrella crate** — it re-exports the public API of the entire Praxis framework. Start here.

## What Praxis does

Praxis collapses the fragmented Solana test toolchain (LiteSVM, Mollusk, Surfpool) into a single backend-agnostic API and adds three capabilities that don't exist as integrated tooling elsewhere:

- **Invariant fuzzer** — adversarial account/transaction generation with proptest shrinking
- **CU profiler** — per-instruction compute-unit flame graphs and commit diffs
- **Pre-audit report** — Markdown + JSON output aggregating all findings

## Quick start

```toml
[dev-dependencies]
praxis = { package = "e-okelloh-praxis", version = "0.1" }
```

```rust
use praxis::prelude::*;

#[invariant_test]
fn no_unauthorized_withdrawal(ctx: &mut Ctx) {
    ctx.with_mutations([Mutation::MissingSigner]);
    let result = ctx.run_instruction("release");
    ctx.assert_invariant(|_| result.success == false, "signer must be required");
}
```

Then run:

```bash
cargo install e-okelloh-praxis-cli --bin praxis
praxis fuzz
```

## Crate structure

| Crate | Purpose |
|---|---|
| `e-okelloh-praxis-core` | `Svm` trait, `NormalIdl`, core types |
| `e-okelloh-praxis-svm-litesvm` | LiteSVM backend (default, fastest) |
| `e-okelloh-praxis-svm-mollusk` | Mollusk backend (CU isolation) |
| `e-okelloh-praxis-svm-surfpool` | Surfpool backend (mainnet fork) |
| `e-okelloh-praxis-idl` | Anchor / Codama / Shank IDL ingestion |
| `e-okelloh-praxis-gen` | Adversarial generators and mutation strategies |
| `e-okelloh-praxis-fuzz` | Invariant fuzzer engine |
| `e-okelloh-praxis-profile` | CU profiler |
| `e-okelloh-praxis-checks` | AC / FD / CPI / T22 check pack |
| `e-okelloh-praxis-report` | Pre-audit report generator |
| `e-okelloh-praxis-macros` | `#[invariant_test]`, `#[profile]` proc macros |
| `e-okelloh-praxis-cli` | `praxis` CLI binary |

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
