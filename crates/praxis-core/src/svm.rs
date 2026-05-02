//! `Svm` trait and associated result/capability types.
use std::collections::HashMap;

use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    transaction::{Transaction, TransactionError},
};

/// Result of executing a single transaction against an SVM backend.
#[derive(Debug, Clone)]
pub struct ExecResult {
    pub success: bool,
    pub cu_consumed: u64,
    pub logs: Vec<String>,
    pub return_data: Option<Vec<u8>>,
    pub error: Option<TransactionError>,
    pub mutated_accounts: Vec<Pubkey>,
}

/// Feature flags reported by each backend.
#[derive(Debug, Clone, Copy)]
pub struct SvmCapabilities {
    pub mainnet_fork: bool,
    pub cu_introspection: bool,
    pub cheatcodes: bool,
    pub parallel_safe: bool,
}

/// Point-in-time snapshot of all accounts and clock state.
/// Cloning the account map is O(state-size) by design.
#[derive(Debug, Clone)]
pub struct SvmSnapshot {
    pub accounts: HashMap<Pubkey, Account>,
    pub slot: u64,
    pub timestamp: i64,
}

/// Backend-agnostic SVM abstraction. Keep this trait small — every new
/// method must be implemented by all backends.
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
