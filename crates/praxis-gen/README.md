# praxis-gen

Adversarial account generators and mutation strategies for the Praxis Solana fuzzing framework.

[![Crates.io](https://img.shields.io/crates/v/e-okelloh-praxis-gen.svg)](https://crates.io/crates/e-okelloh-praxis-gen)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](#license)

## What this crate does

Given a `NormalIdl`, this crate generates account sets and transactions that a real attacker might send. Each mutation strategy targets a specific Solana bug class.

## Mutation strategies (Phase 1)

| Strategy | What it does | Targets |
|---|---|---|
| `MissingSigner` | Drops the `is_signer` flag | Signer-check bypass |
| `WrongOwner` | Replaces owner with a random program ID | Owner-check bypass |
| `WrongPdaSeeds` | Substitutes a PDA derived from wrong seeds | PDA spoofing |
| `FakeProgram` | Replaces a CPI target with an attacker program | Arbitrary CPI |
| `DuplicateAccount` | Aliases two account slots to the same key | Aliasing logic bugs |
| `UninitializedRead` | Passes a freshly created account where init is expected | Init-check bypass |

## Design

Every mutation strategy is a **pure function**: `(NormalInstruction, AccountSet, Seed) -> AccountSet`. No global state, no RNG without an explicit seed. This guarantees deterministic replay.

```rust
use praxis_gen::mutation::Mutation;
use praxis_gen::account::AccountSpawner;

let spawner = AccountSpawner::from_idl(&idl);
let accounts = spawner.spawn_valid(&instruction, seed)?;
let mutated = Mutation::MissingSigner.apply(&instruction, accounts, seed);
```

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
