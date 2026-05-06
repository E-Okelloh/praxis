//! `AccountSpawner` — generates valid accounts satisfying IDL constraints.
use std::sync::Arc;

use praxis_core::{
    AccountConstraint, IxAccountMeta, KnownProgram, NormalInstruction, PdaProgram, SeedComponent,
};
use solana_sdk::{account::Account, pubkey::Pubkey, signature::Keypair};

use crate::{rng::Rng, AccountEntry, AccountSet};

// Minimum lamports to make an account rent-exempt (approximate).
const LAMPORTS: u64 = 1_000_000_000;

/// Generates a valid `AccountSet` for the given instruction and seed.
///
/// Accounts are spawned in slot order. Account-derived PDA seeds are resolved
/// by looking up previously spawned accounts in the same call.
pub struct AccountSpawner;

impl AccountSpawner {
    pub fn spawn(ix: &NormalInstruction, program_id: Pubkey, seed: u64) -> AccountSet {
        let mut rng = Rng::new(seed);
        let mut set = AccountSet::new();

        for meta in &ix.accounts {
            let entry = spawn_entry(meta, program_id, &set, &mut rng);
            set.push(meta.name.clone(), entry);
        }

        set
    }
}

fn spawn_entry(
    meta: &IxAccountMeta,
    program_id: Pubkey,
    existing: &AccountSet,
    rng: &mut Rng,
) -> AccountEntry {
    match &meta.constraint {
        // ── Fixed address ────────────────────────────────────────────────────
        Some(AccountConstraint::Fixed(pk)) => AccountEntry::new(*pk, default_lamports()),

        // ── Known program ────────────────────────────────────────────────────
        Some(AccountConstraint::Program(known)) => {
            let pk = known_program_id(known);
            AccountEntry::new(pk, executable_account(pk))
        }

        // ── PDA ──────────────────────────────────────────────────────────────
        Some(AccountConstraint::Pda(rule)) => {
            let seeds: Vec<Vec<u8>> = rule
                .seeds
                .iter()
                .map(|s| resolve_seed(s, existing, rng))
                .collect();
            let seed_slices: Vec<&[u8]> = seeds.iter().map(|s| s.as_slice()).collect();
            let deriving_program = match &rule.program {
                PdaProgram::Self_ => program_id,
                PdaProgram::Known(k) => known_program_id(k),
                PdaProgram::AccountField(name) => existing
                    .get(name)
                    .map(|e| e.pubkey)
                    .unwrap_or(program_id),
            };
            let (pda, _bump) = Pubkey::find_program_address(&seed_slices, &deriving_program);
            AccountEntry::new(pda, default_lamports())
        }

        // ── Owner check ──────────────────────────────────────────────────────
        Some(AccountConstraint::Owner(owner)) => {
            let pk = rng.next_pubkey();
            let mut acc = default_lamports();
            acc.owner = *owner;
            AccountEntry::new(pk, acc)
        }

        // ── Init (allocate space) ────────────────────────────────────────────
        Some(AccountConstraint::Init { space, .. }) => {
            let kp = Arc::new(Keypair::try_from(rng.next_bytes::<64>().as_slice()).unwrap_or_else(|_| Keypair::new()));
            let mut acc = default_lamports();
            acc.data = vec![0u8; *space as usize];
            acc.owner = program_id;
            AccountEntry::new_signer(kp, acc)
        }

        // ── Signer ───────────────────────────────────────────────────────────
        Some(AccountConstraint::Signer) => {
            let kp = Arc::new(Keypair::try_from(rng.next_bytes::<64>().as_slice()).unwrap_or_else(|_| Keypair::new()));
            AccountEntry::new_signer(kp, default_lamports())
        }

        // ── Signer flag without explicit constraint ───────────────────────────
        None if meta.signer => {
            let kp = Arc::new(Keypair::try_from(rng.next_bytes::<64>().as_slice()).unwrap_or_else(|_| Keypair::new()));
            AccountEntry::new_signer(kp, default_lamports())
        }

        // ── Uninitialized ────────────────────────────────────────────────────
        Some(AccountConstraint::Uninitialized) => {
            let pk = rng.next_pubkey();
            AccountEntry::new(pk, Account::default())
        }

        // ── Raw / unknown — fall through to a plain random account ───────────
        Some(AccountConstraint::Raw(_)) | None => {
            let pk = rng.next_pubkey();
            AccountEntry::new(pk, default_lamports())
        }
    }
}

fn resolve_seed(component: &SeedComponent, existing: &AccountSet, rng: &mut Rng) -> Vec<u8> {
    match component {
        SeedComponent::Literal(bytes) => bytes.clone(),
        SeedComponent::AccountKey(name) => existing
            .get(name)
            .map(|e| e.pubkey.to_bytes().to_vec())
            .unwrap_or_else(|| rng.next_pubkey().to_bytes().to_vec()),
        SeedComponent::AccountField(path) => {
            // Resolve "account.field" — take the account pubkey as a proxy
            // since we don't have field-level data yet.
            let account_name = path.split('.').next().unwrap_or(path);
            existing
                .get(account_name)
                .map(|e| e.pubkey.to_bytes().to_vec())
                .unwrap_or_else(|| rng.next_bytes::<32>().to_vec())
        }
        SeedComponent::InstructionArg(_) => rng.next_bytes::<8>().to_vec(),
    }
}

fn known_program_id(k: &KnownProgram) -> Pubkey {
    match k {
        KnownProgram::System => solana_sdk::system_program::ID,
        KnownProgram::Token => spl_token_id(),
        KnownProgram::Token2022 => spl_token_2022_id(),
        KnownProgram::AssociatedToken => spl_associated_token_id(),
        KnownProgram::MetadataProgram => metadata_program_id(),
    }
}

fn default_lamports() -> Account {
    Account {
        lamports: LAMPORTS,
        ..Account::default()
    }
}

fn executable_account(pk: Pubkey) -> Account {
    Account {
        lamports: LAMPORTS,
        owner: pk,
        executable: true,
        ..Account::default()
    }
}

// Hard-coded well-known program IDs (avoids pulling in spl crates for now).
fn spl_token_id() -> Pubkey {
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        .parse()
        .expect("static pubkey")
}

fn spl_token_2022_id() -> Pubkey {
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        .parse()
        .expect("static pubkey")
}

fn spl_associated_token_id() -> Pubkey {
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJe1bTJL2sR4"
        .parse()
        .expect("static pubkey")
}

fn metadata_program_id() -> Pubkey {
    "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
        .parse()
        .expect("static pubkey")
}

#[cfg(test)]
mod tests {
    use super::*;
    use praxis_core::{IxAccountMeta, NormalInstruction, PdaRule, PdaProgram, SeedComponent};

    fn make_ix(accounts: Vec<IxAccountMeta>) -> NormalInstruction {
        NormalInstruction {
            name: "test".into(),
            discriminator: vec![1, 2, 3, 4, 5, 6, 7, 8],
            accounts,
            args: vec![],
        }
    }

    #[test]
    fn signer_slot_has_keypair() {
        let ix = make_ix(vec![IxAccountMeta {
            name: "authority".into(),
            writable: true,
            signer: true,
            optional: false,
            constraint: Some(AccountConstraint::Signer),
        }]);
        let set = AccountSpawner::spawn(&ix, Pubkey::new_unique(), 42);
        let entry = set.get("authority").unwrap();
        assert!(entry.keypair.is_some());
        assert_eq!(entry.keypair.as_ref().unwrap().pubkey(), entry.pubkey);
    }

    #[test]
    fn pda_slot_derives_correct_address() {
        let program_id = Pubkey::new_unique();
        let ix = make_ix(vec![
            IxAccountMeta {
                name: "payer".into(),
                writable: true,
                signer: true,
                optional: false,
                constraint: Some(AccountConstraint::Signer),
            },
            IxAccountMeta {
                name: "vault".into(),
                writable: true,
                signer: false,
                optional: false,
                constraint: Some(AccountConstraint::Pda(PdaRule {
                    program: PdaProgram::Self_,
                    seeds: vec![SeedComponent::Literal(b"vault".to_vec())],
                    bump_field: None,
                })),
            },
        ]);
        let set = AccountSpawner::spawn(&ix, program_id, 1);
        let vault = set.get("vault").unwrap();
        let (expected, _) = Pubkey::find_program_address(&[b"vault"], &program_id);
        assert_eq!(vault.pubkey, expected);
    }

    #[test]
    fn fixed_address_slot_matches() {
        let fixed_pk = Pubkey::new_unique();
        let ix = make_ix(vec![IxAccountMeta {
            name: "token_prog".into(),
            writable: false,
            signer: false,
            optional: false,
            constraint: Some(AccountConstraint::Fixed(fixed_pk)),
        }]);
        let set = AccountSpawner::spawn(&ix, Pubkey::new_unique(), 7);
        assert_eq!(set.get("token_prog").unwrap().pubkey, fixed_pk);
    }

    #[test]
    fn spawn_is_deterministic() {
        let program_id = Pubkey::new_unique();
        let ix = make_ix(vec![IxAccountMeta {
            name: "acc".into(),
            writable: false,
            signer: false,
            optional: false,
            constraint: None,
        }]);
        let set_a = AccountSpawner::spawn(&ix, program_id, 99);
        let set_b = AccountSpawner::spawn(&ix, program_id, 99);
        assert_eq!(set_a.get("acc").unwrap().pubkey, set_b.get("acc").unwrap().pubkey);
    }
}
