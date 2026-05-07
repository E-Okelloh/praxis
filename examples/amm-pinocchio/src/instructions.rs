use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
};

use crate::{
    error::AmmError,
    state::{Pool, POOL_DISCRIMINATOR, POOL_SIZE},
};

// ── initialize ────────────────────────────────────────────────────────────────

/// Accounts:
/// 0. `[writable]` pool PDA
/// 1. `[]`         mint_a
/// 2. `[]`         mint_b
/// 3. `[signer]`   authority / payer
/// 4. `[]`         system program
pub fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _data: &[u8],
) -> ProgramResult {
    let [pool_ai, mint_a_ai, mint_b_ai, authority_ai, _system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !authority_ai.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Derive expected PDA.
    let (expected_pda, bump) = Pubkey::find_program_address(
        &[b"pool", mint_a_ai.key().as_ref(), mint_b_ai.key().as_ref()],
        program_id,
    );
    if pool_ai.key() != &expected_pda {
        return Err(ProgramError::InvalidArgument);
    }

    // Write pool state.
    let mut data = pool_ai.try_borrow_mut_data()?;
    if data.len() < POOL_SIZE {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let pool = unsafe { &mut *(data.as_mut_ptr() as *mut Pool) };
    pool.discriminator = POOL_DISCRIMINATOR;
    pool.mint_a = *mint_a_ai.key();
    pool.mint_b = *mint_b_ai.key();
    pool.reserve_a = 0;
    pool.reserve_b = 0;
    pool.lp_supply = 0;
    pool.bump = bump;

    Ok(())
}

// ── add_liquidity ─────────────────────────────────────────────────────────────

/// Deposit `amount_a` token A and proportional token B, receive LP shares.
///
/// Accounts:
/// 0. `[writable]` pool PDA
/// 1. `[writable]` depositor token-A account
/// 2. `[writable]` depositor token-B account
/// 3. `[writable]` pool vault-A
/// 4. `[writable]` pool vault-B
/// 5. `[signer]`   authority
/// 6. `[]`         token program
///
/// Data: `amount_a: u64 (LE)`, `min_lp: u64 (LE)`
pub fn process_add_liquidity(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let [pool_ai, _tok_a_ai, _tok_b_ai, _vault_a, _vault_b, authority_ai, _tok_prog] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !authority_ai.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let amount_a = read_u64(data, 0)?;
    let min_lp = read_u64(data, 8)?;

    let mut pool_data = pool_ai.try_borrow_mut_data()?;
    let pool = unsafe { &mut *(pool_data.as_mut_ptr() as *mut Pool) };

    if pool.discriminator != POOL_DISCRIMINATOR {
        return Err(AmmError::InvalidPoolState.into());
    }

    // Compute LP shares using constant-product formula.
    let lp_minted = if pool.lp_supply == 0 {
        // Initial deposit: LP = sqrt(a * b) approximated as amount_a.
        amount_a
    } else {
        amount_a
            .checked_mul(pool.lp_supply)
            .and_then(|n| n.checked_div(pool.reserve_a.max(1)))
            .ok_or(AmmError::Overflow)?
    };

    if lp_minted < min_lp {
        return Err(AmmError::SlippageExceeded.into());
    }

    pool.reserve_a = pool.reserve_a.checked_add(amount_a).ok_or(AmmError::Overflow)?;
    pool.lp_supply = pool.lp_supply.checked_add(lp_minted).ok_or(AmmError::Overflow)?;

    Ok(())
}

// ── swap ──────────────────────────────────────────────────────────────────────

/// Swap `amount_in` of token A for token B (x*y=k).
///
/// **BUG (planted):** does not verify that the depositor's token-A account
/// has `mint == pool.mint_a`. Any token account can be passed in.
///
/// Accounts:
/// 0. `[writable]` pool PDA
/// 1. `[writable]` user source token account (token A)
/// 2. `[writable]` user destination token account (token B)
/// 3. `[writable]` pool vault-A
/// 4. `[writable]` pool vault-B
/// 5. `[signer]`   user authority
/// 6. `[]`         token program
///
/// Data: `amount_in: u64 (LE)`, `min_out: u64 (LE)`
pub fn process_swap(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let [pool_ai, _src_ai, _dst_ai, _vault_a, _vault_b, authority_ai, _tok_prog] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !authority_ai.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let amount_in = read_u64(data, 0)?;
    let min_out = read_u64(data, 8)?;

    let mut pool_data = pool_ai.try_borrow_mut_data()?;
    let pool = unsafe { &mut *(pool_data.as_mut_ptr() as *mut Pool) };

    if pool.discriminator != POOL_DISCRIMINATOR {
        return Err(AmmError::InvalidPoolState.into());
    }

    // x*y = k, output = y * amount_in / (x + amount_in)
    let numerator = pool.reserve_b
        .checked_mul(amount_in)
        .ok_or(AmmError::Overflow)?;
    let denominator = pool.reserve_a
        .checked_add(amount_in)
        .ok_or(AmmError::Overflow)?;
    let amount_out = numerator.checked_div(denominator.max(1)).ok_or(AmmError::Overflow)?;

    if amount_out < min_out {
        return Err(AmmError::SlippageExceeded.into());
    }
    if amount_out > pool.reserve_b {
        return Err(AmmError::InsufficientLiquidity.into());
    }

    pool.reserve_a = pool.reserve_a.checked_add(amount_in).ok_or(AmmError::Overflow)?;
    pool.reserve_b = pool.reserve_b.checked_sub(amount_out).ok_or(AmmError::Overflow)?;

    Ok(())
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64, ProgramError> {
    data.get(offset..offset + 8)
        .and_then(|b| b.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or(ProgramError::InvalidInstructionData)
}
