//! Adversarial account generators, mutation strategies, tx composition, and shrinking.
#![deny(unsafe_code)]

pub mod account;
pub mod mutation;
pub mod rng;
pub mod shrink;
pub mod tx;

pub use account::AccountSpawner;
pub use mutation::MutationStrategy;
pub use tx::TxComposer;

use std::collections::HashMap;
use std::sync::Arc;

use solana_sdk::{account::Account, pubkey::Pubkey, signature::Keypair, signer::Signer as _};

/// Runtime state for a single account slot in a transaction.
#[derive(Clone)]
pub struct AccountEntry {
    pub pubkey: Pubkey,
    pub account: Account,
    /// Present when this slot must sign the transaction.
    pub keypair: Option<Arc<Keypair>>,
}

impl std::fmt::Debug for AccountEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccountEntry")
            .field("pubkey", &self.pubkey)
            .field("lamports", &self.account.lamports)
            .field("is_signer", &self.keypair.is_some())
            .finish()
    }
}

impl AccountEntry {
    pub fn new(pubkey: Pubkey, account: Account) -> Self {
        Self { pubkey, account, keypair: None }
    }

    pub fn new_signer(keypair: Arc<Keypair>, account: Account) -> Self {
        let pubkey = keypair.pubkey();
        Self { pubkey, account, keypair: Some(keypair) }
    }
}

/// Ordered collection of account slots for one instruction invocation.
/// Slot order matches the instruction's account list.
#[derive(Debug, Clone)]
pub struct AccountSet {
    /// (slot_name, entry) in instruction order.
    pub slots: Vec<(String, AccountEntry)>,
    /// Keypairs for any signers not already in slots (e.g. fee payer).
    pub extra_signers: HashMap<Pubkey, Arc<Keypair>>,
}

impl AccountSet {
    pub fn new() -> Self {
        Self { slots: Vec::new(), extra_signers: HashMap::new() }
    }

    pub fn push(&mut self, name: impl Into<String>, entry: AccountEntry) {
        self.slots.push((name.into(), entry));
    }

    pub fn get(&self, name: &str) -> Option<&AccountEntry> {
        self.slots.iter().find(|(n, _)| n == name).map(|(_, e)| e)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut AccountEntry> {
        self.slots.iter_mut().find(|(n, _)| n == name).map(|(_, e)| e)
    }

    /// Collect all keypairs (slot signers + extra signers).
    pub fn all_keypairs(&self) -> Vec<Arc<Keypair>> {
        let mut kps: Vec<Arc<Keypair>> = self
            .slots
            .iter()
            .filter_map(|(_, e)| e.keypair.clone())
            .collect();
        kps.extend(self.extra_signers.values().cloned());
        kps
    }

    /// Return the pubkey of the first writable signer slot (used as fee payer).
    pub fn fee_payer(&self) -> Option<Pubkey> {
        self.slots
            .iter()
            .find(|(_, e)| e.keypair.is_some() && e.account.lamports > 0)
            .map(|(_, e)| e.pubkey)
    }
}

impl Default for AccountSet {
    fn default() -> Self {
        Self::new()
    }
}
