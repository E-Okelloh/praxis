use pinocchio::pubkey::Pubkey;

/// Pool state stored in a PDA account.
/// Layout: 1 (discriminator) + 32 + 32 + 8 + 8 + 8 + 1 = 90 bytes
#[repr(C)]
pub struct Pool {
    pub discriminator: u8,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub lp_supply: u64,
    pub bump: u8,
}

pub const POOL_SIZE: usize = std::mem::size_of::<Pool>();
pub const POOL_DISCRIMINATOR: u8 = 0xAA;
