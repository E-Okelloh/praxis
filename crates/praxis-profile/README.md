# praxis-profile

Compute-unit (CU) profiler for Solana programs — flame graphs, commit diffs, and regression detection.

[![Crates.io](https://img.shields.io/crates/v/e-okelloh-praxis-profile.svg)](https://crates.io/crates/e-okelloh-praxis-profile)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](#license)

## What this crate does

Uses the Mollusk backend for per-instruction CU isolation and wraps `inferno` to render SVG flame graphs attributing CU cost to individual functions.

## CLI usage

```bash
# Generate a flame graph for your program
praxis profile

# Compare CU usage between current code and a baseline commit
praxis profile diff <git-ref>
```

Output is an SVG file in `.praxis/profiles/`.

## Programmatic usage

```rust
use praxis_profile::Profiler;

let mut profiler = Profiler::new(mollusk_backend);
profiler.record(tx)?;
profiler.render_svg(".praxis/profiles/latest.svg")?;
```

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
