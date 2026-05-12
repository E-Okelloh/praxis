# praxis-fuzz

Invariant fuzzer engine for the Praxis Solana testing framework.

[![Crates.io](https://img.shields.io/crates/v/e-okelloh-praxis-fuzz.svg)](https://crates.io/crates/e-okelloh-praxis-fuzz)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](#license)

## What this crate does

This is the core fuzz loop. It:

1. Snapshots the SVM state
2. Generates an adversarial transaction via `praxis-gen`
3. Executes it against the target program
4. Checks all registered invariants
5. On violation: shrinks via proptest, persists the finding to `.praxis/findings/<id>.json`
6. Restores the snapshot and continues

Throughput: **10,000+ iterations/second** with the default LiteSVM backend on an 8-core machine using `rayon` for data-parallel workers.

## Invariant API

```rust
use praxis_fuzz::Ctx;

let mut ctx = Ctx::builder()
    .backend(LiteSvmBackend::new())
    .idl("target/idl/escrow.json")
    .mutations([Mutation::MissingSigner, Mutation::WrongOwner])
    .seed(0xDEAD_BEEF)
    .build()?;

ctx.invariant("authority must sign", |state| {
    state.last_result().success == false
});

ctx.run(50_000)?;
```

## Deterministic replay

Every finding includes a hex seed. Reproduce any finding with:

```bash
praxis replay --seed <hex>
```

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
