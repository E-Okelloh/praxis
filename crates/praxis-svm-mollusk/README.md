# praxis-svm-mollusk

Mollusk backend for the Praxis `Svm` trait — used for per-instruction compute-unit isolation and CU profiling.

[![Crates.io](https://img.shields.io/crates/v/e-okelloh-praxis-svm-mollusk.svg)](https://crates.io/crates/e-okelloh-praxis-svm-mollusk)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](#license)

## When to use this backend

Use Mollusk when you need accurate, isolated CU measurements. The `praxis profile` command selects Mollusk automatically because it separates each instruction's CU cost from syscall and transaction overhead.

## Capabilities

```
mainnet_fork:     false
cu_introspection: true   ← per-instruction CU isolation
cheatcodes:       false
parallel_safe:    true
```

## Usage

```rust
use praxis_svm_mollusk::MolluskBackend;
use praxis_core::Svm;

let mut svm = MolluskBackend::new(program_id, "path/to/program.so");
let result = svm.execute(tx);
println!("CU used: {}", result.cu_consumed);
```

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
