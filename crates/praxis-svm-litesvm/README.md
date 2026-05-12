# praxis-svm-litesvm

LiteSVM backend for the Praxis `Svm` trait — the default, fastest in-memory Solana execution environment.

[![Crates.io](https://img.shields.io/crates/v/e-okelloh-praxis-svm-litesvm.svg)](https://crates.io/crates/e-okelloh-praxis-svm-litesvm)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](#license)

## Why LiteSVM for fuzzing

LiteSVM runs entirely in-process with no network I/O. A single fuzz loop iteration is a function call, not an RPC round-trip. This backend achieves **10,000+ iterations/second** on an 8-core machine, making it the default for `praxis fuzz`.

## Capabilities

```
mainnet_fork:     false
cu_introspection: true
cheatcodes:       true
parallel_safe:    true
```

## Snapshot / restore

`snapshot()` clones the relevant account map (O(state-size), not O(slot-history)). The fuzzer calls this before each generated transaction and `restore()` on failure to prevent state contamination between iterations.

## Usage

```rust
use praxis_svm_litesvm::LiteSvmBackend;
use praxis_core::Svm;

let mut svm = LiteSvmBackend::new();
svm.set_account(&program_id, program_account);
let result = svm.execute(tx);
```

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
