# Check Pack Reference

`praxis check` runs the check pack against a program's IDL and optionally its runtime behaviour.  Checks are identified by ID and organised into families.

## Running checks

```bash
# Static checks from IDL:
praxis check --idl target/idl/escrow.json

# Filter to a family:
praxis check --idl target/idl/escrow.json --filter AC

# Fail CI on high+ findings:
praxis check --idl target/idl/escrow.json --fail-on high
```

## Check families

### AC — Access Control

| ID | Type | Severity | Description |
|----|------|----------|-------------|
| `AC-001` | Static | High | Every account whose name contains `authority`, `owner`, `admin`, `manager`, or `controller` must have the `signer` flag or a `Signer` constraint. |
| `AC-002` | Static | Medium | Every non-exempt writable account should have an explicit owner or PDA constraint to prevent accepting arbitrary program-owned data. |

**AC-001 example finding:**
```
[AC-001] HIGH — Instruction `release`: account `authority` looks like an authority but has no signer constraint (release::authority)
```

**AC-002 example finding:**
```
[AC-002] MEDIUM — Instruction `cancel`: account `escrow_state` has no explicit owner constraint (cancel::escrow_state)
```

### CPI — Cross-Program Invocation

| ID | Type | Severity | Description |
|----|------|----------|-------------|
| `CPI-001` | Runtime | High | A CPI targeted a program ID not in the whitelist. Triggered by the fuzzer when it observes an unexpected CPI destination. |

### FD — Feed / Freshness

| ID | Type | Severity | Description |
|----|------|----------|-------------|
| `FD-001` | Runtime | High | Protocol invariants violated after N skipped slots (slot-warp fuzzing). |
| `FD-002` | Runtime | High | Oracle `last_update_slot` is older than the configured staleness threshold. |
| `FD-003` | Runtime | High | Oracle confidence/price ratio exceeds the configured threshold. |

**FD-002 example:**
```rust
// In your invariant test:
use praxis_checks::check_fd_002_staleness;
let finding = check_fd_002_staleness("pyth_sol", last_slot, current_slot, 100);
```

**FD-003 example:**
```rust
use praxis_checks::check_fd_003_confidence;
let finding = check_fd_003_confidence("pyth_sol", price, confidence, 0.10);
```

### T22 — Token-2022 / Transfer Hook

| ID | Type | Severity | Description |
|----|------|----------|-------------|
| `T22-001` | Runtime | Critical | A Transfer Hook issued a CPI back into the Token-2022 program on the same mint — potential re-entrancy. |
| `T22-002` | Static + Runtime | High | An `ExtraAccountMetaList` account has no PDA seed constraint, allowing injection of malicious extra accounts. |
| `T22-003` | Runtime | Critical | ZK proof inputs do not match expected ciphertexts. |

## Severity levels

| Level | Meaning | Default CI behaviour |
|-------|---------|----------------------|
| `critical` | Actively exploitable | Always fails |
| `high` | Likely exploitable | Fails by default (`--fail-on high`) |
| `medium` | Warrants review | Does not fail by default |
| `info` | Low confidence | Informational only |

## Programmatic API

```rust
use praxis_checks::{check_ac_001, check_ac_002};
use praxis_idl::parse_anchor_idl;

let idl = parse_anchor_idl("target/idl/my_program.json").unwrap();
let findings = [check_ac_001(&idl), check_ac_002(&idl)].concat();

for f in &findings {
    println!("[{}] {:?} — {}", f.check_id, f.severity, f.message);
}
```

## Extending the check pack

New checks should live in `crates/praxis-checks/src/<family>.rs` and be exported from `lib.rs`.  Every check must:

1. Return `Vec<CheckFinding>` (empty = pass).
2. Have at least one unit test that fires and at least one that passes.
3. Be documented with the check ID, severity, and a description of the bug class it targets.
