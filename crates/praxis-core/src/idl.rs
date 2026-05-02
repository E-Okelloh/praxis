//! Normalised IDL types shared across all IDL backends (Anchor, Codama, Shank).
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Top-level normalised representation of a program IDL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalIdl {
    pub name: String,
    pub version: String,
    pub program_id: Pubkey,
    pub instructions: Vec<NormalInstruction>,
}

/// A single instruction in the normalised IDL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalInstruction {
    pub name: String,
    /// 8-byte Anchor discriminator or equivalent.
    pub discriminator: Vec<u8>,
    pub accounts: Vec<IxAccountMeta>,
    pub args: Vec<InstructionArg>,
}

/// Per-account metadata with constraint information used by the generators.
/// Named `IxAccountMeta` to avoid collision with `solana_sdk::instruction::AccountMeta`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IxAccountMeta {
    pub name: String,
    pub writable: bool,
    pub signer: bool,
    pub optional: bool,
    pub constraint: Option<AccountConstraint>,
}

/// Typed constraints on an account slot, derived from the IDL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountConstraint {
    /// Account must be owned by this program.
    Owner(Pubkey),
    /// Account is a PDA derived according to the given rule.
    Pda(PdaRule),
    /// Account must equal a specific known pubkey.
    Fixed(Pubkey),
    /// Account must be the system program, token program, etc.
    Program(KnownProgram),
    /// Account must be a signer (redundant with `signer` flag but explicit).
    Signer,
    /// Account must be initialised in this instruction (space + rent).
    Init { space: u64, payer: String },
    /// Account must be zeroed / not yet initialised.
    Uninitialized,
    /// Custom constraint expressed as a raw string (passthrough for unknown constraints).
    Raw(String),
}

/// A PDA derivation rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdaRule {
    pub program: PdaProgram,
    pub seeds: Vec<SeedComponent>,
    /// Canonical bump seed account field name, if stored.
    pub bump_field: Option<String>,
}

/// Which program derives the PDA.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PdaProgram {
    /// The program being tested.
    Self_,
    /// A known external program.
    Known(KnownProgram),
    /// Arbitrary program identified by account slot name.
    AccountField(String),
}

/// A single component of a PDA seed vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeedComponent {
    /// Constant UTF-8 string literal, e.g. `b"escrow"`.
    Literal(Vec<u8>),
    /// Value taken from a named account field.
    AccountField(String),
    /// Value taken from an instruction argument.
    InstructionArg(String),
    /// The account's own pubkey bytes.
    AccountKey(String),
}

/// Well-known Solana programs referenced in constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnownProgram {
    System,
    Token,
    Token2022,
    AssociatedToken,
    MetadataProgram,
}

/// A typed instruction argument.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionArg {
    pub name: String,
    pub ty: IdlType,
}

/// Minimal type representation for instruction arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IdlType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    Bytes,
    String,
    PublicKey,
    Option(Box<IdlType>),
    Vec(Box<IdlType>),
    Struct(String),
    Enum(String),
}
