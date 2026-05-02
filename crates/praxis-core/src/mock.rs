//! In-memory `MockSvm` for unit-testing higher layers without a real backend.
use std::collections::HashMap;

use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    transaction::Transaction,
};

use crate::svm::{ExecResult, Svm, SvmCapabilities, SvmSnapshot};

/// A minimal in-memory SVM whose `execute` returns a caller-supplied canned result.
/// Useful for testing `praxis-fuzz` and `praxis-gen` in isolation.
pub struct MockSvm {
    accounts: HashMap<Pubkey, Account>,
    slot: u64,
    timestamp: i64,
    canned: ExecResult,
}

impl MockSvm {
    pub fn new(canned: ExecResult) -> Self {
        Self {
            accounts: HashMap::new(),
            slot: 0,
            timestamp: 0,
            canned,
        }
    }

    /// Convenience constructor that always reports success with zero CU.
    pub fn always_ok() -> Self {
        Self::new(ExecResult {
            success: true,
            cu_consumed: 0,
            logs: vec![],
            return_data: None,
            error: None,
            mutated_accounts: vec![],
        })
    }

    /// Convenience constructor that always reports failure.
    pub fn always_fail() -> Self {
        use solana_sdk::transaction::TransactionError;
        Self::new(ExecResult {
            success: false,
            cu_consumed: 0,
            logs: vec![],
            return_data: None,
            error: Some(TransactionError::AccountNotFound),
            mutated_accounts: vec![],
        })
    }

    /// Override the canned result at runtime.
    pub fn set_canned(&mut self, result: ExecResult) {
        self.canned = result;
    }
}

impl Svm for MockSvm {
    fn execute(&mut self, _tx: Transaction) -> ExecResult {
        self.canned.clone()
    }

    fn account(&self, pk: &Pubkey) -> Option<Account> {
        self.accounts.get(pk).cloned()
    }

    fn set_account(&mut self, pk: &Pubkey, acc: Account) {
        self.accounts.insert(*pk, acc);
    }

    fn snapshot(&self) -> SvmSnapshot {
        SvmSnapshot {
            accounts: self.accounts.clone(),
            slot: self.slot,
            timestamp: self.timestamp,
        }
    }

    fn restore(&mut self, snap: &SvmSnapshot) {
        self.accounts = snap.accounts.clone();
        self.slot = snap.slot;
        self.timestamp = snap.timestamp;
    }

    fn warp_slot(&mut self, slot: u64) {
        self.slot = slot;
    }

    fn warp_timestamp(&mut self, ts: i64) {
        self.timestamp = ts;
    }

    fn capabilities(&self) -> SvmCapabilities {
        SvmCapabilities {
            mainnet_fork: false,
            cu_introspection: false,
            cheatcodes: true,
            parallel_safe: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_ok_returns_success() {
        let mut svm = MockSvm::always_ok();
        let tx = Transaction::default();
        assert!(svm.execute(tx).success);
    }

    #[test]
    fn always_fail_returns_error() {
        let mut svm = MockSvm::always_fail();
        let tx = Transaction::default();
        let result = svm.execute(tx);
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn snapshot_restore_roundtrip() {
        let mut svm = MockSvm::always_ok();
        let pk = Pubkey::new_unique();
        let acc = Account {
            lamports: 1_000_000,
            ..Account::default()
        };
        svm.set_account(&pk, acc.clone());
        let snap = svm.snapshot();

        // mutate
        svm.set_account(&pk, Account::default());
        assert_eq!(svm.account(&pk).unwrap().lamports, 0);

        // restore
        svm.restore(&snap);
        assert_eq!(svm.account(&pk).unwrap().lamports, 1_000_000);
    }

    #[test]
    fn warp_slot_and_timestamp() {
        let mut svm = MockSvm::always_ok();
        svm.warp_slot(42);
        svm.warp_timestamp(1_700_000_000);
        let snap = svm.snapshot();
        assert_eq!(snap.slot, 42);
        assert_eq!(snap.timestamp, 1_700_000_000);
    }
}
