use pinocchio::program_error::ProgramError;

#[derive(Clone, Debug)]
pub enum AmmError {
    InvalidPoolState = 0,
    SlippageExceeded = 1,
    InsufficientLiquidity = 2,
    Overflow = 3,
}

impl From<AmmError> for ProgramError {
    fn from(e: AmmError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
