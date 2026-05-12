# praxis-core

Core types, the `Svm` trait, and `NormalIdl` schema — the foundation of the Praxis Solana testing framework.

[![Crates.io](https://img.shields.io/crates/v/e-okelloh-praxis-core.svg)](https://crates.io/crates/e-okelloh-praxis-core)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](#license)

## The `Svm` trait

Every backend in Praxis implements this trait. It is intentionally small — adding methods requires work in every backend.

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

## `NormalIdl`

A backend-agnostic IDL representation parsed from Anchor, Codama, or Shank sources. The generator and fuzzer layers operate exclusively on `NormalIdl` — they have no knowledge of the original IDL format.

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
