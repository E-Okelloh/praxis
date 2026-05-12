# praxis-idl

IDL ingestion for the Praxis Solana fuzzing framework — parses Anchor, Codama, and Shank IDLs into a unified `NormalIdl`.

[![Crates.io](https://img.shields.io/crates/v/e-okelloh-praxis-idl.svg)](https://crates.io/crates/e-okelloh-praxis-idl)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](#license)

## Supported IDL formats

| Format | Entry point | Typical program type |
|---|---|---|
| Anchor 0.30+ JSON IDL | `parse_anchor_idl(path)` | Anchor programs |
| Codama JSON IDL | `parse_codama_idl(path)` | Pinocchio / framework-agnostic |
| Shank annotations | `parse_shank_idl(path)` | Steel, bare `solana-program` |

All three paths produce a `NormalIdl` — the single type the rest of Praxis operates on.

## Account constraints

Every account constraint in the source IDL is mapped to an `AccountConstraint` enum variant, including:

- Signer / writable flags
- PDA seeds (literal, account-derived, bump)
- Owner checks
- Token mint / authority relationships

## Usage

```rust
use praxis_idl::parse_anchor_idl;

let idl = parse_anchor_idl("target/idl/escrow.json")?;
println!("{} instructions", idl.instructions.len());
```

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
