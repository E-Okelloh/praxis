//! CU-sensitive constant-product AMM implemented with Pinocchio.
//!
//! # Architecture
//!
//! Three instructions:
//! - `initialize` — create a Pool account holding reserve A and B token mints
//! - `add_liquidity` — deposit token A + B, receive LP shares
//! - `swap` — exchange token A for B (or B for A) using x*y=k invariant
//!
//! # CU profile (approximate, single-threaded LiteSVM)
//! | Instruction    | ~CU |
//! |----------------|-----|
//! | `initialize`   | 1 200 |
//! | `add_liquidity`| 3 800 |
//! | `swap`         | 4 500 |
//!
//! # Planted bug (for Praxis T22/FD checks)
//! `swap` does NOT validate that `token_a_account.mint == pool.mint_a`.
//! An attacker can pass any token account whose balance will be drained.

#![deny(unsafe_code)]

use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
};

mod error;
mod instructions;
mod state;

pub use error::AmmError;
pub use state::Pool;

// Re-export for tests.
pub use instructions::{process_add_liquidity, process_initialize, process_swap};

/// 8-byte discriminators for each instruction.
pub const IX_INITIALIZE: u8 = 0;
pub const IX_ADD_LIQUIDITY: u8 = 1;
pub const IX_SWAP: u8 = 2;

#[cfg(not(feature = "no-entrypoint"))]
pinocchio::program_entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (&tag, rest) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    match tag {
        IX_INITIALIZE => instructions::process_initialize(program_id, accounts, rest),
        IX_ADD_LIQUIDITY => instructions::process_add_liquidity(program_id, accounts, rest),
        IX_SWAP => instructions::process_swap(program_id, accounts, rest),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
