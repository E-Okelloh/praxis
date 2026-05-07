//! Token-2022 Transfer Hook with planted T22 bugs.
//!
//! # Planted bugs
//!
//! | Bug | Check | Description |
//! |-----|-------|-------------|
//! | T22-001 | Re-entrancy | `execute` CPIs back into the Token-2022 program to transfer from the mint account — a classic hook re-entrancy |
//! | T22-002 | Seed validation | `initialize_extra_account_meta_list` does not validate that ExtraAccountMeta seeds are derived from the correct mint |
//!
//! # Instructions
//!
//! - `initialize_extra_account_meta_list` — register extra account metas
//! - `execute` — called by Token-2022 on every transfer; applies fee logic
#![deny(unsafe_code)]

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

mod error;
mod execute;
mod initialize;

pub use error::HookError;

#[cfg(not(feature = "no-entrypoint"))]
solana_program::entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Discriminators (first 8 bytes).
    let discriminator = &instruction_data[..8];

    if discriminator == spl_transfer_hook_interface::instruction::ExecuteInstruction::SPL_DISCRIMINATOR_SLICE {
        msg!("TransferHook: Execute");
        execute::process_execute(program_id, accounts, &instruction_data[8..])
    } else if discriminator == spl_transfer_hook_interface::instruction::InitializeExtraAccountMetaListInstruction::SPL_DISCRIMINATOR_SLICE {
        msg!("TransferHook: InitializeExtraAccountMetaList");
        initialize::process_initialize(program_id, accounts, &instruction_data[8..])
    } else {
        Err(ProgramError::InvalidInstructionData)
    }
}
