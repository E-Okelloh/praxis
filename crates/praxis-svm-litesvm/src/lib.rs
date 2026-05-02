//! LiteSVM backend — implements the `Svm` trait using `litesvm::LiteSVM`.
#![deny(unsafe_code)]

use std::collections::HashMap;

use litesvm::LiteSVM;
use praxis_core::{ExecResult, Svm, SvmCapabilities, SvmSnapshot};
use solana_sdk::{
    account::Account,
    clock::Clock,
    pubkey::Pubkey,
    transaction::Transaction,
};

pub use litesvm::error::LiteSVMError;

/// Wraps `litesvm::LiteSVM` and implements the backend-agnostic `Svm` trait.
///
/// Snapshot/restore is O(account-count): the full account map is cloned,
/// not a slot-history chain.
pub struct LiteSvmBackend {
    inner: LiteSVM,
}

impl LiteSvmBackend {
    pub fn new() -> Self {
        Self {
            inner: LiteSVM::new(),
        }
    }

    /// Airdrop lamports to an address (convenience wrapper).
    pub fn airdrop(&mut self, to: &Pubkey, lamports: u64) -> Result<(), litesvm::types::FailedTransactionMetadata> {
        self.inner.airdrop(to, lamports).map(|_| ())
    }

    /// Deploy a compiled program from raw bytes.
    pub fn add_program(&mut self, program_id: Pubkey, program_bytes: &[u8]) {
        self.inner.add_program(program_id, program_bytes);
    }

    /// Deploy a compiled program from a `.so` file path.
    pub fn add_program_from_file(
        &mut self,
        program_id: Pubkey,
        path: &std::path::Path,
    ) -> Result<(), LiteSVMError> {
        self.inner.add_program_from_file(program_id, path)
    }

    /// Expose the inner `LiteSVM` for backend-specific operations.
    pub fn inner(&self) -> &LiteSVM {
        &self.inner
    }

    /// Expose the inner `LiteSVM` mutably.
    pub fn inner_mut(&mut self) -> &mut LiteSVM {
        &mut self.inner
    }
}

impl Default for LiteSvmBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Svm for LiteSvmBackend {
    fn execute(&mut self, tx: Transaction) -> ExecResult {
        match self.inner.send_transaction(tx) {
            Ok(meta) => ExecResult {
                success: true,
                cu_consumed: meta.compute_units_consumed,
                logs: meta.logs,
                return_data: {
                    let d = meta.return_data.data;
                    if d.is_empty() { None } else { Some(d) }
                },
                error: None,
                mutated_accounts: vec![],
            },
            Err(failed) => ExecResult {
                success: false,
                cu_consumed: failed.meta.compute_units_consumed,
                logs: failed.meta.logs,
                return_data: {
                    let d = failed.meta.return_data.data;
                    if d.is_empty() { None } else { Some(d) }
                },
                error: Some(failed.err),
                mutated_accounts: vec![],
            },
        }
    }

    fn account(&self, pk: &Pubkey) -> Option<Account> {
        self.inner.get_account(pk)
    }

    fn set_account(&mut self, pk: &Pubkey, acc: Account) {
        // Ignore error — set_account only fails for special system accounts
        let _ = self.inner.set_account(*pk, acc);
    }

    fn snapshot(&self) -> SvmSnapshot {
        let clock = self.inner.get_sysvar::<Clock>();
        let accounts: HashMap<Pubkey, Account> = self
            .inner
            .accounts_db()
            .inner
            .iter()
            .map(|(pk, asd)| (*pk, Account::from(asd.clone())))
            .collect();
        SvmSnapshot {
            accounts,
            slot: clock.slot,
            timestamp: clock.unix_timestamp,
        }
    }

    fn restore(&mut self, snap: &SvmSnapshot) {
        // Rebuild from scratch to avoid accumulating stale program-cache entries.
        self.inner = LiteSVM::new();
        for (pk, acc) in &snap.accounts {
            let _ = self.inner.set_account(*pk, acc.clone());
        }
        // Restore clock state via the sysvar.
        let mut clock = self.inner.get_sysvar::<Clock>();
        clock.slot = snap.slot;
        clock.unix_timestamp = snap.timestamp;
        self.inner.set_sysvar(&clock);
    }

    fn warp_slot(&mut self, slot: u64) {
        self.inner.warp_to_slot(slot);
    }

    fn warp_timestamp(&mut self, ts: i64) {
        let mut clock = self.inner.get_sysvar::<Clock>();
        clock.unix_timestamp = ts;
        self.inner.set_sysvar(&clock);
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
    use solana_sdk::{
        message::Message,
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
        system_instruction,
        transaction::Transaction,
    };

    fn make_transfer_tx(
        svm: &mut LiteSvmBackend,
        from: &Keypair,
        to: &Pubkey,
        lamports: u64,
    ) -> Transaction {
        let blockhash = svm.inner.latest_blockhash();
        let ix = system_instruction::transfer(&from.pubkey(), to, lamports);
        let msg = Message::new(&[ix], Some(&from.pubkey()));
        Transaction::new(&[from], msg, blockhash)
    }

    #[test]
    fn execute_transfer_reports_cu_and_logs() {
        let mut svm = LiteSvmBackend::new();
        let from = Keypair::new();
        let to = Pubkey::new_unique();

        svm.airdrop(&from.pubkey(), 10_000_000).unwrap();
        let tx = make_transfer_tx(&mut svm, &from, &to, 1_000);
        let result = svm.execute(tx);

        assert!(result.success, "transfer should succeed: {:?}", result.error);
        assert!(result.cu_consumed > 0, "should report non-zero CUs");
        assert!(svm.account(&to).map(|a| a.lamports) == Some(1_000));
    }

    #[test]
    fn capabilities_flags_are_correct() {
        let svm = LiteSvmBackend::new();
        let caps = svm.capabilities();
        assert!(!caps.mainnet_fork);
        assert!(caps.cu_introspection);
        assert!(caps.cheatcodes);
        assert!(caps.parallel_safe);
    }

    #[test]
    fn snapshot_restore_preserves_balances() {
        let mut svm = LiteSvmBackend::new();
        let from = Keypair::new();
        let to = Pubkey::new_unique();

        svm.airdrop(&from.pubkey(), 10_000_000).unwrap();

        // Take snapshot before the transfer
        let snap = svm.snapshot();
        let before_from = svm.account(&from.pubkey()).unwrap().lamports;

        // Execute a transfer — mutates state
        let tx = make_transfer_tx(&mut svm, &from, &to, 1_000);
        let result = svm.execute(tx);
        assert!(result.success);

        let after_from = svm.account(&from.pubkey()).unwrap().lamports;
        assert!(after_from < before_from, "lamports should have decreased");

        // Restore — state must match original
        svm.restore(&snap);
        let restored_from = svm.account(&from.pubkey()).unwrap().lamports;
        assert_eq!(
            restored_from, before_from,
            "restore must recover exact pre-transfer balance"
        );
        // Destination should no longer exist (or have zero lamports)
        let restored_to_lamports = svm.account(&to).map(|a| a.lamports).unwrap_or(0);
        assert_eq!(restored_to_lamports, 0, "destination should not exist after restore");
    }

    #[test]
    fn warp_slot_and_timestamp_reflected_in_snapshot() {
        let mut svm = LiteSvmBackend::new();
        svm.warp_slot(999);
        svm.warp_timestamp(1_700_000_000);
        let snap = svm.snapshot();
        assert_eq!(snap.slot, 999);
        assert_eq!(snap.timestamp, 1_700_000_000);
    }
}
