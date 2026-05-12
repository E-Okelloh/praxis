# praxis-svm-surfpool

Surfpool backend for the Praxis `Svm` trait — mainnet-fork integration testing against live on-chain state.

[![Crates.io](https://img.shields.io/crates/v/e-okelloh-praxis-svm-surfpool.svg)](https://crates.io/crates/e-okelloh-praxis-svm-surfpool)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](#license)

## When to use this backend

Use Surfpool for pre-release testing that requires real mainnet state — live oracle prices, live token balances, live program state. Your invariant tests written against LiteSVM run identically here.

**Note:** this backend requires a running Surfpool RPC endpoint and is gated behind the `surfpool` feature flag in CI.

## Capabilities

```
mainnet_fork:     true   ← fetches live account state
cu_introspection: false
cheatcodes:       false
parallel_safe:    false
```

## Async runtime

This is the only Praxis crate that uses `tokio`. All other crates are sync. Surfpool tests must be gated in CI with the `--features surfpool` flag.

## Usage

```toml
[dev-dependencies]
praxis-svm-surfpool = { package = "e-okelloh-praxis-svm-surfpool", version = "0.1", features = ["surfpool"] }
```

```rust
use praxis_svm_surfpool::SurfpoolBackend;
let mut svm = SurfpoolBackend::connect("http://localhost:8899").await?;
```

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
