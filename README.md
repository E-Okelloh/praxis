# Praxis

> Rust-native testing, fuzzing, and profiling framework for Solana programs.

Praxis collapses the fragmented Solana test toolchain into a **single backend-agnostic API** and adds line-level CU profiling, Solana-aware property-based invariant fuzzing, and a pre-audit report generator.

## Status

Phase 1 in progress — core runtime and invariant fuzzer.

## Crates

| Crate | Description | Status |
|---|---|---|
| `praxis` | Umbrella public crate | Phase 1 |
| `praxis-core` | Core types, `Svm` trait, `NormalIdl` | Phase 1 |
| `praxis-idl` | IDL ingestion (Anchor, Codama, Shank) | Phase 1 |
| `praxis-svm-litesvm` | LiteSVM backend | Phase 1 |
| `praxis-svm-mollusk` | Mollusk backend | Phase 2 |
| `praxis-svm-surfpool` | Surfpool backend | Phase 3 |
| `praxis-gen` | Adversarial generators + shrinker | Phase 1 |
| `praxis-fuzz` | Invariant fuzzer engine | Phase 1 |
| `praxis-profile` | CU profiler | Phase 2 |
| `praxis-checks` | Check pack (AC/FD/T22) | Phase 2 |
| `praxis-report` | Markdown + JSON reports | Phase 3 |
| `praxis-macros` | Proc macros | Phase 1 |
| `praxis-cli` | CLI binary (`praxis`) | Phase 1 |

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
