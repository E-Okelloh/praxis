# praxis-macros

Proc macros for the Praxis Solana testing framework: `#[invariant_test]` and `#[profile]`.

[![Crates.io](https://img.shields.io/crates/v/e-okelloh-praxis-macros.svg)](https://crates.io/crates/e-okelloh-praxis-macros)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](#license)

## Design principle

**Functions first, macros as sugar.** Every macro here expands to a plain function call form. If you prefer not to use macros, you can call the underlying API directly.

## `#[invariant_test]`

Marks a function as a Praxis invariant test, discoverable by the test runner.

```rust
use praxis_macros::invariant_test;

#[invariant_test]
fn authority_must_sign(ctx: &mut Ctx) {
    ctx.with_mutations([Mutation::MissingSigner]);
    let result = ctx.run_instruction("release");
    ctx.assert_invariant(|_| !result.success, "signer must be required");
}
```

Expands to: a function registered via `inventory` that `praxis test` discovers at runtime.

## `#[profile]`

Wraps a test function to record CU usage via the Mollusk backend.

```rust
#[profile]
fn measure_swap_cu(ctx: &mut Ctx) {
    ctx.run_instruction("swap");
}
```

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
