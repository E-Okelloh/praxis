# praxis-checks

Static and runtime security check pack for Solana programs — access control, flash loan, CPI, and Token-2022 checks.

[![Crates.io](https://img.shields.io/crates/v/e-okelloh-praxis-checks.svg)](https://crates.io/crates/e-okelloh-praxis-checks)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](#license)

## Checks implemented

| Check ID | What it asserts | Method |
|---|---|---|
| `AC-001` | Every authority parameter has a signer constraint | Static + runtime |
| `AC-002` | Every deserialised account has an explicit owner check | Runtime fuzz |
| `CPI-001` | All CPIs target whitelisted program IDs | Runtime fuzz |
| `FD-001` | Protocol invariants hold across N skipped slots | Runtime, slot warp |
| `FD-002` | Pyth/Switchboard `last_update_slot` ≤ N slots old | Runtime |
| `FD-003` | Pyth confidence interval rejects above threshold | Runtime |
| `T22-001` | Transfer Hook does not CPI back into the same mint | Runtime fuzz |
| `T22-002` | All `ExtraAccountMetaList` seeds are validated | Runtime fuzz |
| `T22-003` | ZK proof inputs match expected ciphertexts | Runtime |

## Usage

```bash
# Run all enabled checks, fast
praxis check
```

Or programmatically:

```rust
use praxis_checks::{CheckPack, CheckId};

let pack = CheckPack::from_idl(&idl)
    .enable([CheckId::AC001, CheckId::AC002, CheckId::CPI001]);
let findings = pack.run(&mut svm)?;
```

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
