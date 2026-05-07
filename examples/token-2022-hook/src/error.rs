use solana_program::program_error::ProgramError;

#[derive(Clone, Debug)]
pub enum HookError {
    InvalidAccountMeta = 0,
    Unauthorized = 1,
}

impl From<HookError> for ProgramError {
    fn from(e: HookError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
