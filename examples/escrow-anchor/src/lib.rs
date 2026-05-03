//! Escrow program — reference target for Praxis fuzzing.
//!
//! Contains 3 deliberately planted security bugs for e2e testing:
//!
//! - **Bug 1 (MissingSigner)**: `release` does not verify that `authority`
//!   is a signer. Any account can impersonate the authority.
//!
//! - **Bug 2 (WrongOwner)**: `release` deserialises `escrow_state` without
//!   checking that the account is owned by this program. Attacker can pass a
//!   crafted account owned by a different program.
//!
//! - **Bug 3 (WrongPdaSeeds)**: `cancel` does not verify that `escrow_state`
//!   is the PDA derived from the expected seeds. Any account at the right
//!   address (or a spoofed PDA) can be passed.
#![deny(unsafe_code)]

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

// ─── State ───────────────────────────────────────────────────────────────────

/// Discriminator bytes stored at the start of every EscrowState account.
pub const ESCROW_DISCRIMINATOR: [u8; 8] = [0x65, 0x73, 0x63, 0x72, 0x6f, 0x77, 0x00, 0x01];

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct EscrowState {
    pub discriminator: [u8; 8],
    /// The party that deposited the lamports and can cancel.
    pub depositor: Pubkey,
    /// The party that can claim by calling `release`.
    pub authority: Pubkey,
    /// Lamports held in the vault.
    pub amount: u64,
    /// PDA bump.
    pub bump: u8,
}

impl EscrowState {
    pub const SIZE: usize = 8 + 32 + 32 + 8 + 1;

    pub fn seeds<'a>(depositor: &'a Pubkey) -> Vec<&'a [u8]> {
        vec![b"escrow", depositor.as_ref()]
    }
}

// ─── Instructions ────────────────────────────────────────────────────────────

#[repr(u8)]
pub enum EscrowInstruction {
    /// Create the escrow. Accounts: [depositor(signer,writable), escrow_state(writable,pda), vault(writable,pda), system_program]
    Create = 0,
    /// Release to authority. Accounts: [authority, escrow_state(writable), vault(writable,pda), recipient(writable)]
    Release = 1,
    /// Cancel — return to depositor. Accounts: [depositor(signer,writable), escrow_state(writable), vault(writable,pda)]
    Cancel = 2,
}

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (&tag, _rest) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    match tag {
        0 => process_create(program_id, accounts),
        1 => process_release(program_id, accounts),
        2 => process_cancel(program_id, accounts),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

// ─── Create ──────────────────────────────────────────────────────────────────

fn process_create(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let iter = &mut accounts.iter();
    let depositor = next_account_info(iter)?;
    let escrow_state_info = next_account_info(iter)?;
    let vault_info = next_account_info(iter)?;
    let authority_key = next_account_info(iter)?; // passed for storage only
    let system_program = next_account_info(iter)?;

    // Parse amount from remaining data (simplified: hard-coded to 1_000_000 for tests)
    let amount: u64 = 1_000_000;

    // Verify depositor is signer.
    if !depositor.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (escrow_pda, bump) = Pubkey::find_program_address(
        &[b"escrow", depositor.key.as_ref()],
        program_id,
    );
    if escrow_pda != *escrow_state_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    let (vault_pda, vault_bump) = Pubkey::find_program_address(
        &[b"vault", depositor.key.as_ref()],
        program_id,
    );
    if vault_pda != *vault_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    let rent = Rent::get()?;
    let state_space = EscrowState::SIZE;
    let state_lamports = rent.minimum_balance(state_space);

    // Create escrow_state PDA account.
    invoke_signed(
        &system_instruction::create_account(
            depositor.key,
            escrow_state_info.key,
            state_lamports,
            state_space as u64,
            program_id,
        ),
        &[depositor.clone(), escrow_state_info.clone(), system_program.clone()],
        &[&[b"escrow", depositor.key.as_ref(), &[bump]]],
    )?;

    // Create vault PDA account and fund it.
    let vault_lamports = rent.minimum_balance(0) + amount;
    invoke_signed(
        &system_instruction::create_account(
            depositor.key,
            vault_info.key,
            vault_lamports,
            0,
            program_id,
        ),
        &[depositor.clone(), vault_info.clone(), system_program.clone()],
        &[&[b"vault", depositor.key.as_ref(), &[vault_bump]]],
    )?;

    // Write state.
    let state = EscrowState {
        discriminator: ESCROW_DISCRIMINATOR,
        depositor: *depositor.key,
        authority: *authority_key.key,
        amount,
        bump,
    };
    let data = borsh::to_vec(&state).map_err(|_| ProgramError::BorshIoError("serialise".into()))?;
    escrow_state_info.data.borrow_mut().copy_from_slice(&data);

    Ok(())
}

// ─── Release ─────────────────────────────────────────────────────────────────
//
// BUG 1: authority is NOT checked for is_signer.
// BUG 2: escrow_state owner is NOT checked against program_id.

fn process_release(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let iter = &mut accounts.iter();
    let authority_info = next_account_info(iter)?;
    let escrow_state_info = next_account_info(iter)?;
    let vault_info = next_account_info(iter)?;
    let recipient_info = next_account_info(iter)?;

    // ~~~ BUG 1: missing signer check ~~~
    // Should be: if !authority_info.is_signer { return Err(ProgramError::MissingRequiredSignature); }

    // ~~~ BUG 2: missing owner check ~~~
    // Should be: if escrow_state_info.owner != program_id { return Err(ProgramError::IllegalOwner); }

    let state = EscrowState::try_from_slice(&escrow_state_info.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Check discriminator.
    if state.discriminator != ESCROW_DISCRIMINATOR {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify authority key matches stored authority.
    if state.authority != *authority_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    // Derive vault PDA and verify.
    let (_vault_pda, vault_bump) = Pubkey::find_program_address(
        &[b"vault", state.depositor.as_ref()],
        program_id,
    );

    // Transfer vault lamports to recipient.
    let vault_balance = vault_info.lamports();
    **vault_info.try_borrow_mut_lamports()? -= vault_balance;
    **recipient_info.try_borrow_mut_lamports()? += vault_balance;

    // Close escrow_state (return rent to recipient).
    let state_balance = escrow_state_info.lamports();
    **escrow_state_info.try_borrow_mut_lamports()? -= state_balance;
    **recipient_info.try_borrow_mut_lamports()? += state_balance;

    let _ = vault_bump; // used in derivation above
    Ok(())
}

// ─── Cancel ──────────────────────────────────────────────────────────────────
//
// BUG 3: escrow_state PDA seeds are NOT verified. Any account at that address
//        (or any account whose key happens to match) is accepted.

fn process_cancel(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let iter = &mut accounts.iter();
    let depositor_info = next_account_info(iter)?;
    let escrow_state_info = next_account_info(iter)?;
    let vault_info = next_account_info(iter)?;

    if !depositor_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // ~~~ BUG 3: missing PDA seed verification ~~~
    // Should be:
    //   let (expected_pda, _) = Pubkey::find_program_address(
    //       &[b"escrow", depositor_info.key.as_ref()], program_id);
    //   if expected_pda != *escrow_state_info.key { return Err(ProgramError::InvalidArgument); }

    let state = EscrowState::try_from_slice(&escrow_state_info.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if state.discriminator != ESCROW_DISCRIMINATOR {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify depositor matches stored depositor.
    if state.depositor != *depositor_info.key {
        return Err(ProgramError::InvalidArgument);
    }

    // Drain vault to depositor.
    let vault_balance = vault_info.lamports();
    **vault_info.try_borrow_mut_lamports()? -= vault_balance;
    **depositor_info.try_borrow_mut_lamports()? += vault_balance;

    // Close escrow_state.
    let state_balance = escrow_state_info.lamports();
    **escrow_state_info.try_borrow_mut_lamports()? -= state_balance;
    **depositor_info.try_borrow_mut_lamports()? += state_balance;

    let _ = program_id;
    Ok(())
}
