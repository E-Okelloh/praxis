# Writing Invariants

An **invariant** is a predicate over post-execution state that must hold for every transaction the fuzzer generates.  If any generated transaction causes the predicate to return `false`, Praxis records a finding with a deterministic seed so you can replay it.

## Quickstart

```rust
use praxis_fuzz::{Ctx, Finding};
use praxis_svm_litesvm::LiteSvmBackend;

#[test]
fn escrow_invariants() {
    let svm = LiteSvmBackend::new();
    let mut ctx = Ctx::new(svm, idl, program_id, seed);

    // Register an invariant: after every transaction, the vault must not
    // be empty if the escrow is still open.
    ctx.invariant("vault_never_drained_early", |svm, result| {
        if !result.success { return true; } // Only check successful txs.
        let vault = svm.account(&vault_pk).unwrap_or_default();
        vault.lamports >= RENT_EXEMPT
    });

    let findings = ctx.run();
    assert!(findings.is_empty(), "violations found: {findings:#?}");
}
```

## Anatomy of a `Ctx`

```rust
Ctx::new(svm, idl, program_id, seed)
    .with_mutations(vec![MissingSigner, WrongOwner, WrongPdaSeeds])
    .with_iterations(50_000)
    .invariant("name", |svm, result| bool)
    .run()
```

| Method | Purpose |
|--------|---------|
| `with_mutations` | Choose which adversarial strategies to apply |
| `with_iterations` | Number of fuzz iterations (default: 1 000) |
| `invariant` | Register a named predicate |
| `run` | Execute the fuzz loop; returns `Vec<Finding>` |

## Invariant predicate signature

```rust
fn(&dyn Svm, &ExecResult) -> bool
```

- **`true`** — invariant holds; no finding.
- **`false`** — invariant violated; Praxis records a finding.

The SVM is post-execution: all account mutations from the transaction are visible.

## Common patterns

### 1. Check that a protected account was not modified

```rust
ctx.invariant("authority_unchanged", |svm, _result| {
    svm.account(&authority_pk)
        .map(|a| a.owner == expected_owner)
        .unwrap_or(true)
});
```

### 2. Check that funds cannot be extracted without signer

```rust
ctx.invariant("funds_intact_without_signer", |svm, result| {
    if result.success {
        svm.account(&vault_pk).map(|a| a.lamports).unwrap_or(0) >= initial_lamports
    } else {
        true // Failed txs don't drain the vault
    }
});
```

### 3. Verify protocol arithmetic invariant

```rust
ctx.invariant("constant_product", |svm, result| {
    if !result.success { return true; }
    let pool = read_pool(svm, &pool_pk);
    pool.reserve_a * pool.reserve_b >= initial_k
});
```

## Replaying a finding

Every finding is saved to `.praxis/findings/<id>.json` and includes a `replay_cmd`:

```bash
praxis replay --seed <hex-id>
```

This re-runs the exact mutation sequence that triggered the violation.

## Using `#[invariant_test]`

The proc macro expands to a standard `#[test]` that the normal test runner discovers:

```rust
use praxis_macros::invariant_test;

#[invariant_test(program = "escrow", idl = "target/idl/escrow.json")]
fn escrow_signer_invariant(ctx: &mut Ctx) {
    ctx.invariant("release_requires_authority", |svm, result| {
        // ...
    });
}
```

## Choosing mutations

| Strategy | Tests for |
|----------|-----------|
| `MissingSigner` | Signer-check bypass |
| `WrongOwner` | Owner-check bypass |
| `WrongPdaSeeds` | PDA spoofing |
| `FakeProgram` | Arbitrary CPI |
| `DuplicateAccount` | Account aliasing |
| `UninitializedRead` | Init-check bypass |

Start with all six, then narrow to the ones most relevant to your program.
