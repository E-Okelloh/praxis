//! Mollusk-compatible backend — implements the `Svm` trait with per-instruction
//! CU tracking, built on top of `LiteSvmBackend`.
//!
//! ## Why not wrap `mollusk_svm::Mollusk` directly?
//!
//! All published `mollusk-svm` versions (≥0.4) pull in Agave 4.x split crates
//! (`solana-pubkey v4.x`, `solana-account v3.x`, `solana-instruction v3.x`),
//! which are **different Rust types** from the workspace's `solana-sdk = "2.x"`.
//! The two families cannot cross-call without unsafe transmutes.  Additionally,
//! `mollusk_svm::Mollusk` contains `Rc<RefCell<…>>` fields, making it
//! `!Send + !Sync` and therefore incompatible with `trait Svm: Send + Sync`.
//!
//! ## Design
//!
//! `MolluskBackend` wraps [`LiteSvmBackend`] for all `Svm` trait methods and
//! additionally tracks per-instruction CU breakdowns for the profiler layer.
//! When `praxis-profile` needs Mollusk-native CU data it drives `mollusk-svm`
//! directly via a raw-bytes API that bridges the two Agave generations.
#![deny(unsafe_code)]

use praxis_core::{ExecResult, Svm, SvmCapabilities, SvmSnapshot};
use praxis_svm_litesvm::LiteSvmBackend;
use solana_sdk::{account::Account, pubkey::Pubkey, transaction::Transaction};

/// Wraps [`LiteSvmBackend`] and adds per-instruction CU tracking.
///
/// For CU flame-graph profiling, the profiler layer (in `praxis-profile`)
/// drives `mollusk-svm` directly with Agave-native types.  This struct is the
/// `Svm`-trait-compatible handle the rest of the framework uses.
pub struct MolluskBackend {
    inner: LiteSvmBackend,
    /// CU consumed by each instruction in the most recent [`execute`] call,
    /// in instruction order.  Reset on every `execute`.
    last_per_ix_cu: Vec<u64>,
}

impl MolluskBackend {
    /// Create a new, empty backend.
    pub fn new() -> Self {
        Self {
            inner: LiteSvmBackend::new(),
            last_per_ix_cu: Vec::new(),
        }
    }

    /// Load a compiled BPF program.
    pub fn add_program(&mut self, program_id: Pubkey, elf: &[u8]) {
        self.inner.add_program(program_id, elf);
    }

    /// Per-instruction CU breakdown from the most recent `execute` call.
    ///
    /// Currently reports the total transaction CU split evenly across
    /// instructions as a best-effort attribution; the profiler layer uses the
    /// finer-grained Mollusk API directly for flame-graph data.
    pub fn last_per_ix_cu(&self) -> &[u64] {
        &self.last_per_ix_cu
    }

    /// Direct access to the underlying [`LiteSvmBackend`].
    pub fn inner(&self) -> &LiteSvmBackend {
        &self.inner
    }

    /// Mutable access to the underlying [`LiteSvmBackend`].
    pub fn inner_mut(&mut self) -> &mut LiteSvmBackend {
        &mut self.inner
    }
}

impl Default for MolluskBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Svm for MolluskBackend {
    fn execute(&mut self, tx: Transaction) -> ExecResult {
        let ix_count = tx.message.instructions.len().max(1);
        let result = self.inner.execute(tx);

        // Distribute total CU evenly across instructions as a best-effort
        // estimate.  The profiler layer provides exact per-ix CU via the
        // mollusk-svm API driven with native Agave types.
        let per_ix = result.cu_consumed / ix_count as u64;
        self.last_per_ix_cu = vec![per_ix; ix_count];

        result
    }

    fn account(&self, pk: &Pubkey) -> Option<Account> {
        self.inner.account(pk)
    }

    fn set_account(&mut self, pk: &Pubkey, acc: Account) {
        self.inner.set_account(pk, acc);
    }

    fn snapshot(&self) -> SvmSnapshot {
        self.inner.snapshot()
    }

    fn restore(&mut self, snap: &SvmSnapshot) {
        self.inner.restore(snap);
    }

    fn warp_slot(&mut self, slot: u64) {
        self.inner.warp_slot(slot);
    }

    fn warp_timestamp(&mut self, ts: i64) {
        self.inner.warp_timestamp(ts);
    }

    fn capabilities(&self) -> SvmCapabilities {
        SvmCapabilities {
            mainnet_fork: false,
            cu_introspection: true,
            cheatcodes: true,
            parallel_safe: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    #[test]
    fn capabilities_flags() {
        let backend = MolluskBackend::new();
        let caps = backend.capabilities();
        assert!(!caps.mainnet_fork);
        assert!(caps.cu_introspection);
        assert!(caps.cheatcodes);
        assert!(caps.parallel_safe);
    }

    #[test]
    fn set_and_get_account_roundtrip() {
        let mut backend = MolluskBackend::new();
        let key = Pubkey::new_unique();
        let acc = Account {
            lamports: 42,
            ..Account::default()
        };
        backend.set_account(&key, acc.clone());
        assert_eq!(backend.account(&key).map(|a| a.lamports), Some(42));
    }

    #[test]
    fn snapshot_restore_roundtrip() {
        let mut backend = MolluskBackend::new();
        let key = Pubkey::new_unique();
        backend.set_account(
            &key,
            Account {
                lamports: 100,
                ..Account::default()
            },
        );

        let snap = backend.snapshot();

        backend.set_account(
            &key,
            Account {
                lamports: 999,
                ..Account::default()
            },
        );
        assert_eq!(backend.account(&key).unwrap().lamports, 999);

        backend.restore(&snap);
        assert_eq!(
            backend.account(&key).unwrap().lamports,
            100,
            "restore must recover pre-mutation lamports"
        );
    }

    #[test]
    fn warp_slot_reflected_in_snapshot() {
        let mut backend = MolluskBackend::new();
        backend.warp_slot(777);
        let snap = backend.snapshot();
        assert_eq!(snap.slot, 777);
    }
}
