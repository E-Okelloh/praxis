//! Adversarial mutation strategies.  Each strategy is a pure function:
//! `(NormalInstruction, AccountSet, u64 seed) -> AccountSet`.
use std::sync::Arc;

use praxis_core::{AccountConstraint, NormalInstruction, PdaProgram, PdaRule, SeedComponent};
use solana_sdk::{account::Account, pubkey::Pubkey, signature::Keypair, signer::Signer};

use crate::{rng::Rng, AccountEntry, AccountSet};

/// The six Phase-1 mutation strategies from CLAUDE.md §11.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MutationStrategy {
    /// Drop `is_signer` — tests signer-check bypass.
    MissingSigner,
    /// Replace account owner with a random program ID — tests owner-check bypass.
    WrongOwner,
    /// Substitute a PDA derived from wrong seeds — tests PDA spoofing.
    WrongPdaSeeds,
    /// Replace the program ID in a CPI slot — tests arbitrary CPI.
    FakeProgram,
    /// Alias two account slots to the same pubkey — tests aliasing logic.
    DuplicateAccount,
    /// Pass a freshly-zeroed account where init is expected — tests init-check bypass.
    UninitializedRead,
}

impl MutationStrategy {
    /// Apply this strategy to `accounts`, returning a mutated copy.
    /// All randomness is seeded deterministically from `seed`.
    pub fn apply(
        &self,
        ix: &NormalInstruction,
        accounts: AccountSet,
        seed: u64,
    ) -> AccountSet {
        let mut rng = Rng::new(seed);
        match self {
            MutationStrategy::MissingSigner => missing_signer(ix, accounts),
            MutationStrategy::WrongOwner => wrong_owner(accounts, &mut rng),
            MutationStrategy::WrongPdaSeeds => wrong_pda_seeds(ix, accounts, &mut rng),
            MutationStrategy::FakeProgram => fake_program(accounts, &mut rng),
            MutationStrategy::DuplicateAccount => duplicate_account(accounts, &mut rng),
            MutationStrategy::UninitializedRead => uninitialized_read(ix, accounts, &mut rng),
        }
    }

    /// Human-readable name matching CLAUDE.md taxonomy.
    pub fn name(self) -> &'static str {
        match self {
            Self::MissingSigner => "MissingSigner",
            Self::WrongOwner => "WrongOwner",
            Self::WrongPdaSeeds => "WrongPdaSeeds",
            Self::FakeProgram => "FakeProgram",
            Self::DuplicateAccount => "DuplicateAccount",
            Self::UninitializedRead => "UninitializedRead",
        }
    }

    /// All six Phase-1 strategies, in taxonomy order.
    pub fn all() -> &'static [MutationStrategy] {
        &[
            Self::MissingSigner,
            Self::WrongOwner,
            Self::WrongPdaSeeds,
            Self::FakeProgram,
            Self::DuplicateAccount,
            Self::UninitializedRead,
        ]
    }
}

// ── Individual strategy implementations ─────────────────────────────────────

/// Drop the keypair from the first signer slot so the transaction can still
/// be built but the account won't have `is_signer` set.
fn missing_signer(ix: &NormalInstruction, mut accounts: AccountSet) -> AccountSet {
    // Find the first slot that the IDL says must be a signer.
    if let Some(name) = ix.accounts.iter().find(|a| a.signer).map(|a| a.name.clone()) {
        if let Some(entry) = accounts.get_mut(&name) {
            // Remove keypair so TxComposer won't mark it as a signer.
            entry.keypair = None;
        }
    }
    accounts
}

/// Replace the owner of the first non-signer, non-program account with a random ID.
fn wrong_owner(mut accounts: AccountSet, rng: &mut Rng) -> AccountSet {
    for (_, entry) in &mut accounts.slots {
        if entry.keypair.is_none() && !entry.account.executable {
            entry.account.owner = rng.next_pubkey();
            break;
        }
    }
    accounts
}

/// For the first PDA slot, derive it from a completely different seed set so the
/// address is valid-looking but wrong.
fn wrong_pda_seeds(ix: &NormalInstruction, mut accounts: AccountSet, rng: &mut Rng) -> AccountSet {
    // Find a slot that has a PDA constraint.
    let pda_name = ix.accounts.iter().find_map(|a| {
        if matches!(a.constraint, Some(AccountConstraint::Pda(_))) {
            Some(a.name.clone())
        } else {
            None
        }
    });

    if let Some(name) = pda_name {
        // Derive a fake PDA from a garbage seed under the same program logic.
        let fake_seed = rng.next_bytes::<16>();
        let fake_program = rng.next_pubkey();
        let (wrong_pda, _) = Pubkey::find_program_address(&[&fake_seed], &fake_program);
        if let Some(entry) = accounts.get_mut(&name) {
            entry.pubkey = wrong_pda;
            entry.keypair = None;
        }
    }
    accounts
}

/// Replace a program-account slot (executable=true or `KnownProgram` constraint)
/// with an attacker-controlled program pubkey.
fn fake_program(mut accounts: AccountSet, rng: &mut Rng) -> AccountSet {
    for (_, entry) in &mut accounts.slots {
        if entry.account.executable {
            entry.pubkey = rng.next_pubkey();
            entry.account = Account {
                lamports: 1_000_000,
                executable: true,
                owner: entry.pubkey,
                ..Account::default()
            };
            break;
        }
    }
    accounts
}

/// Alias two distinct account slots to the same pubkey.
fn duplicate_account(mut accounts: AccountSet, rng: &mut Rng) -> AccountSet {
    let len = accounts.slots.len();
    if len < 2 {
        return accounts;
    }
    let i = rng.next_usize_mod(len);
    let j = (i + 1 + rng.next_usize_mod(len - 1)) % len;
    let src_pubkey = accounts.slots[i].1.pubkey;
    accounts.slots[j].1.pubkey = src_pubkey;
    // If the aliased slot had a keypair, remove it (we're not holding the right key).
    accounts.slots[j].1.keypair = None;
    accounts
}

/// Replace the first non-signer data account with a freshly-zeroed account
/// to bypass initialisation checks.
fn uninitialized_read(ix: &NormalInstruction, mut accounts: AccountSet, rng: &mut Rng) -> AccountSet {
    let target = ix.accounts.iter().find(|a| !a.signer).map(|a| a.name.clone());
    if let Some(name) = target {
        if let Some(entry) = accounts.get_mut(&name) {
            // Fresh pubkey + zeroed data = uninitialized account.
            entry.pubkey = rng.next_pubkey();
            entry.account = Account::default();
            entry.keypair = None;
        }
    }
    accounts
}

#[cfg(test)]
mod tests {
    use super::*;
    use praxis_core::{AccountConstraint, IxAccountMeta, NormalInstruction};
    use crate::AccountSpawner;

    fn escrow_ix() -> NormalInstruction {
        NormalInstruction {
            name: "release".into(),
            discriminator: vec![1, 2, 3, 4, 5, 6, 7, 8],
            accounts: vec![
                IxAccountMeta {
                    name: "authority".into(),
                    writable: false,
                    signer: true,
                    optional: false,
                    constraint: Some(AccountConstraint::Signer),
                },
                IxAccountMeta {
                    name: "vault".into(),
                    writable: true,
                    signer: false,
                    optional: false,
                    constraint: None,
                },
            ],
            args: vec![],
        }
    }

    #[test]
    fn missing_signer_drops_keypair() {
        let ix = escrow_ix();
        let program_id = Pubkey::new_unique();
        let original = AccountSpawner::spawn(&ix, program_id, 1);
        assert!(original.get("authority").unwrap().keypair.is_some());

        let mutated = MutationStrategy::MissingSigner.apply(&ix, original, 2);
        assert!(mutated.get("authority").unwrap().keypair.is_none());
        // Pubkey should be unchanged
    }

    #[test]
    fn wrong_owner_changes_owner() {
        let ix = escrow_ix();
        let program_id = Pubkey::new_unique();
        let original = AccountSpawner::spawn(&ix, program_id, 3);
        let original_owner = original.get("vault").unwrap().account.owner;

        let mutated = MutationStrategy::WrongOwner.apply(&ix, original, 4);
        let new_owner = mutated.get("vault").unwrap().account.owner;
        assert_ne!(new_owner, original_owner);
    }

    #[test]
    fn duplicate_account_aliases_two_slots() {
        let ix = escrow_ix();
        let program_id = Pubkey::new_unique();
        let original = AccountSpawner::spawn(&ix, program_id, 5);
        let pk0 = original.slots[0].1.pubkey;
        let pk1 = original.slots[1].1.pubkey;
        assert_ne!(pk0, pk1, "slots should differ before mutation");

        let mutated = MutationStrategy::DuplicateAccount.apply(&ix, original, 6);
        let new_pk0 = mutated.slots[0].1.pubkey;
        let new_pk1 = mutated.slots[1].1.pubkey;
        assert_eq!(new_pk0, new_pk1, "slots should be aliased after mutation");
    }

    #[test]
    fn uninitialized_read_zeroes_data_account() {
        let ix = escrow_ix();
        let program_id = Pubkey::new_unique();
        let original = AccountSpawner::spawn(&ix, program_id, 7);
        let mutated = MutationStrategy::UninitializedRead.apply(&ix, original, 8);
        let vault = mutated.get("vault").unwrap();
        assert_eq!(vault.account.lamports, 0);
        assert!(vault.account.data.is_empty());
    }

    #[test]
    fn mutation_is_deterministic() {
        let ix = escrow_ix();
        let program_id = Pubkey::new_unique();
        let a = AccountSpawner::spawn(&ix, program_id, 9);
        let b = AccountSpawner::spawn(&ix, program_id, 9);
        let ma = MutationStrategy::WrongOwner.apply(&ix, a, 10);
        let mb = MutationStrategy::WrongOwner.apply(&ix, b, 10);
        assert_eq!(
            ma.get("vault").unwrap().account.owner,
            mb.get("vault").unwrap().account.owner
        );
    }
}
