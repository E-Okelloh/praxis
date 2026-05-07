//! Execute — Transfer Hook entry point; T22-001 bug: re-entrant CPI into Token-2022.

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
};

/// **BUG T22-001 planted here:** After applying the transfer fee, this hook
/// issues a CPI back into the Token-2022 program to transfer tokens *from the
/// mint authority account*.  This creates a re-entrancy loop: Token-2022 →
/// Hook → Token-2022, allowing an attacker to drain the mint authority's
/// token balance in a single transaction.
///
/// Accounts (as required by the Transfer Hook interface):
/// 0. `[writable]` source account
/// 1. `[]`         mint
/// 2. `[writable]` destination account
/// 3. `[signer]`   authority (owner of source)
/// 4. `[]`         extra_account_metas
///
/// Data (after discriminator): `amount: u64 (LE)`
pub fn process_execute(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let source_ai      = next_account_info(accounts_iter)?;
    let mint_ai        = next_account_info(accounts_iter)?;
    let destination_ai = next_account_info(accounts_iter)?;
    let authority_ai   = next_account_info(accounts_iter)?;
    let _extra_metas   = next_account_info(accounts_iter)?;

    // Read transfer amount.
    let amount = data
        .get(..8)
        .and_then(|b| b.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or(ProgramError::InvalidInstructionData)?;

    msg!("TransferHook: execute for amount={}", amount);

    // Legitimate fee logic: charge 1% as protocol fee.
    let fee = amount / 100;
    msg!("TransferHook: fee={}", fee);

    // BUG T22-001: issue a re-entrant CPI back into Token-2022 to "collect"
    // the fee by transferring from the mint's freeze authority.
    // A correct implementation would just record the fee in a fee-collector
    // account without issuing any CPI.
    let token_program_id: Pubkey = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        .parse()
        .unwrap();

    // Build a spl_token_2022::instruction::transfer_checked() discriminator.
    // Discriminator bytes for transfer_checked: [12, 0, 0, 0, ...]
    let mut ix_data = vec![12u8]; // transfer_checked discriminator
    ix_data.extend_from_slice(&fee.to_le_bytes()); // amount
    ix_data.push(9u8); // decimals (placeholder)

    let reentrant_ix = Instruction {
        program_id: token_program_id,
        accounts: vec![
            AccountMeta::new(*source_ai.key, false),
            AccountMeta::new_readonly(*mint_ai.key, false),
            AccountMeta::new(*destination_ai.key, false),
            AccountMeta::new_readonly(*authority_ai.key, true),
        ],
        data: ix_data,
    };

    // This CPI re-enters Token-2022 from within a Token-2022 hook — T22-001.
    invoke(
        &reentrant_ix,
        &[
            source_ai.clone(),
            mint_ai.clone(),
            destination_ai.clone(),
            authority_ai.clone(),
        ],
    )?;

    Ok(())
}
