# praxis-cli

The `praxis` CLI — install once, test, fuzz, profile, and audit any Solana program.

[![Crates.io](https://img.shields.io/crates/v/e-okelloh-praxis-cli.svg)](https://crates.io/crates/e-okelloh-praxis-cli)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](#license)

## Installation

```bash
cargo install e-okelloh-praxis-cli --bin praxis
```

## Commands

```bash
praxis init                 # scaffold praxis.toml and .praxis/ directory
praxis test                 # run all #[invariant_test] functions
praxis fuzz                 # long-running adversarial fuzzing
praxis replay --seed <hex>  # deterministically reproduce a finding
praxis profile              # CU flame graph (SVG)
praxis profile diff <ref>   # CU delta vs a git ref
praxis check                # run the AC/FD/CPI/T22 check pack
praxis report               # emit Markdown + JSON pre-audit report
praxis ci                   # all-in-one for CI, exits non-zero on findings
```

## Configuration (`praxis.toml`)

```toml
[program]
name = "escrow"
path = "./target/deploy/escrow.so"
idl  = "./target/idl/escrow.json"

[fuzz]
iterations = 50_000
seed       = 0xDEADBEEF
mutations  = ["MissingSigner", "WrongOwner", "WrongPdaSeeds"]

[checks]
enabled = ["AC-001", "AC-002", "CPI-001"]

[report]
output_dir = "./.praxis/reports"
formats    = ["markdown", "json"]
fail_on    = "high"
```

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
