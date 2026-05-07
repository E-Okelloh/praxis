# Praxis Architecture

Praxis is organised into five horizontal layers. Each layer depends only on layers below it. The `Svm` trait at Layer 2 is the linchpin — keep it small.

```
+---------------------------------------------------------------+
|                      LAYER 5: CLI & REPORT                    |
|   praxis test | praxis fuzz | praxis profile | praxis report  |
+---------------------------------------------------------------+
|                  LAYER 4: HIGHER-ORDER ENGINES                |
|   Invariant Fuzzer | CU Profiler | Check Pack | Diff Engine   |
+---------------------------------------------------------------+
|                LAYER 3: ADVERSARIAL GENERATORS                |
|    Account Mutators | Tx Composer | Seed Strategy | Shrinker  |
+---------------------------------------------------------------+
|              LAYER 2: UNIFIED RUNTIME ABSTRACTION             |
|       trait Svm { execute, account, snapshot, restore }       |
+---------------------------------------------------------------+
|                    LAYER 1: SVM BACKENDS                      |
|   LiteSvmBackend | MolluskBackend | SurfpoolBackend (forking) |
+---------------------------------------------------------------+
|                LAYER 0: SCHEMA / IDL INGESTION                |
|   Anchor IDL parser | Codama parser | Shank annotation parser |
+---------------------------------------------------------------+
```

## Crate map

| Crate | Layer | Purpose |
|-------|-------|---------|
| `praxis-core` | 0–2 | `Svm` trait, `ExecResult`, `NormalIdl`, `MockSvm` |
| `praxis-idl` | 0 | Anchor / Codama / Shank IDL ingestion → `NormalIdl` |
| `praxis-svm-litesvm` | 1 | Fast in-memory backend (default for fuzz loops) |
| `praxis-svm-mollusk` | 1 | CU-introspection backend |
| `praxis-svm-surfpool` | 1 | Mainnet-fork backend (RPC-seeded, gated by `mainnet-fork` feature) |
| `praxis-gen` | 3 | Adversarial account generators + mutation strategies + shrinker |
| `praxis-fuzz` | 4 | Invariant fuzzer engine with proptest shrinking |
| `praxis-profile` | 4 | CU profiler → SVG flame graph via `inferno` |
| `praxis-checks` | 4 | Static + runtime check pack (AC, CPI, FD, T22) |
| `praxis-report` | 5 | Markdown + JSON pre-audit report generator |
| `praxis-macros` | 5 | `#[invariant_test]` proc macro |
| `praxis-cli` | 5 | `praxis` binary |
| `praxis` | 5 | Umbrella crate (re-exports) |

## The `Svm` trait

```rust
pub trait Svm: Send + Sync {
    fn execute(&mut self, tx: Transaction) -> ExecResult;
    fn account(&self, pk: &Pubkey) -> Option<Account>;
    fn set_account(&mut self, pk: &Pubkey, acc: Account);
    fn snapshot(&self) -> SvmSnapshot;
    fn restore(&mut self, snap: &SvmSnapshot);
    fn warp_slot(&mut self, slot: u64);
    fn warp_timestamp(&mut self, ts: i64);
    fn capabilities(&self) -> SvmCapabilities;
}
```

This trait is intentionally small. Every addition is work multiplied across all backends.

## Backend capabilities

| Backend | `mainnet_fork` | `cu_introspection` | `cheatcodes` | `parallel_safe` |
|---------|:-:|:-:|:-:|:-:|
| LiteSvmBackend | ✗ | ✓ | ✓ | ✓ |
| MolluskBackend | ✗ | ✓ | ✓ | ✓ |
| SurfpoolBackend (no fork) | ✗ | ✓ | ✓ | ✓ |
| SurfpoolBackend (forked) | ✓ | ✓ | ✓ | ✓ |

## Data flow: `praxis fuzz`

```
NormalIdl → AccountSpawner → AccountSet
                               │
                    MutationStrategy → mutated AccountSet
                                          │
                                  TxComposer → Transaction
                                                  │
                                        SVM.execute() → ExecResult
                                                            │
                                         Invariant predicates → Finding
                                                            │
                                          proptest shrinker → minimal reproducer
                                                            │
                                               .praxis/findings/<id>.json
```

## Data flow: `praxis report`

```
NormalIdl
  ├─ check_ac_001/002 ──┐
  ├─ check_fd_002/003   │──→ CheckFinding[] → ReportBuilder
  └─ check_t22_001/002  │
                         │
                fuzz findings ──────────────→ ReportBuilder
                profile data ───────────────→ ReportBuilder
                                                    │
                                         Report.to_markdown() → *.md
                                         Report.to_json()     → *.json
```
