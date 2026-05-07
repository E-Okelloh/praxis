//! InitializeExtraAccountMetaList — T22-002 bug: seeds not validated.

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

/// **BUG T22-002 planted here:** The `extra_account_metas` account is accepted
/// without verifying that its address is the PDA derived from the correct mint.
/// An attacker can pass any program-owned account as the extra metas list.
///
/// Accounts:
/// 0. `[writable]` extra_account_metas (SHOULD be PDA derived from mint — NOT CHECKED)
/// 1. `[]`         mint
/// 2. `[signer]`   authority
/// 3. `[signer]`   payer
/// 4. `[]`         system program
pub fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let extra_metas_ai = next_account_info(accounts_iter)?;
    let mint_ai = next_account_info(accounts_iter)?;
    let authority_ai = next_account_info(accounts_iter)?;
    let _payer_ai = next_account_info(accounts_iter)?;
    let _system_prog = next_account_info(accounts_iter)?;

    if !authority_ai.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // BUG T22-002: should verify:
    //   let (expected_pda, _bump) = Pubkey::find_program_address(
    //       &[b"extra-account-metas", mint_ai.key.as_ref()],
    //       program_id,
    //   );
    //   if extra_metas_ai.key != &expected_pda { return Err(...); }
    //
    // Instead we skip this check entirely, accepting any account.

    msg!(
        "Initializing extra account metas for mint {} (seeds NOT validated — T22-002 bug)",
        mint_ai.key
    );

    // Write a minimal TLV account-resolution list header.
    // (In a real implementation this would use spl_tlv_account_resolution.)
    let mut data = extra_metas_ai.try_borrow_mut_data()?;
    if data.len() < 4 {
        return Err(ProgramError::AccountDataTooSmall);
    }
    // Header: discriminator (4 bytes) + count (4 bytes) = 8 bytes.
    data[..4].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
    data[4..8].copy_from_slice(&0u32.to_le_bytes()); // 0 extra accounts

    Ok(())
}
