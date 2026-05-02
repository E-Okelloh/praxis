//! Anchor IDL → NormalIdl mapping.
use std::path::Path;
use std::str::FromStr;

use anchor_lang_idl_spec::{
    Idl, IdlField, IdlInstructionAccount, IdlInstructionAccountItem, IdlPda, IdlSeed, IdlType as AnchorType,
};
use praxis_core::{
    AccountConstraint, IdlType, InstructionArg, IxAccountMeta, KnownProgram, NormalIdl,
    NormalInstruction, PdaProgram, PdaRule, SeedComponent,
};
use solana_sdk::pubkey::Pubkey;

use crate::error::IdlError;

// Well-known program addresses
const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
const ASSOCIATED_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJe1bTJL2sR4";

/// Parse an Anchor IDL JSON file and return a `NormalIdl`.
pub fn parse_anchor_idl(path: &Path) -> Result<NormalIdl, IdlError> {
    let json = std::fs::read_to_string(path)?;
    parse_anchor_idl_str(&json)
}

/// Parse an Anchor IDL from a JSON string slice.
pub fn parse_anchor_idl_str(json: &str) -> Result<NormalIdl, IdlError> {
    let idl: Idl = serde_json::from_str(json)?;
    map_idl(idl)
}

fn map_idl(idl: Idl) -> Result<NormalIdl, IdlError> {
    let program_id = Pubkey::from_str(&idl.address)
        .map_err(|_| IdlError::InvalidAddress(idl.address.clone()))?;

    let instructions = idl
        .instructions
        .into_iter()
        .map(|ix| {
            let flat_accounts = flatten_accounts(ix.accounts);
            let accounts = flat_accounts.into_iter().map(map_account).collect();
            let args = ix.args.into_iter().map(map_field).collect();
            NormalInstruction {
                name: ix.name,
                discriminator: ix.discriminator,
                accounts,
                args,
            }
        })
        .collect();

    Ok(NormalIdl {
        name: idl.metadata.name,
        version: idl.metadata.version,
        program_id,
        instructions,
    })
}

/// Recursively flatten composite account groups into a flat list.
fn flatten_accounts(items: Vec<IdlInstructionAccountItem>) -> Vec<IdlInstructionAccount> {
    items
        .into_iter()
        .flat_map(|item| match item {
            IdlInstructionAccountItem::Single(acc) => vec![acc],
            IdlInstructionAccountItem::Composite(group) => flatten_accounts(group.accounts),
        })
        .collect()
}

fn map_account(acc: IdlInstructionAccount) -> IxAccountMeta {
    let constraint = derive_constraint(&acc);
    IxAccountMeta {
        name: acc.name,
        writable: acc.writable,
        signer: acc.signer,
        optional: acc.optional,
        constraint,
    }
}

/// Derive the most specific `AccountConstraint` we can from the IDL metadata.
fn derive_constraint(acc: &IdlInstructionAccount) -> Option<AccountConstraint> {
    // Fixed address takes priority
    if let Some(addr) = &acc.address {
        if let Some(known) = classify_known_program(addr) {
            return Some(AccountConstraint::Program(known));
        }
        if let Ok(pk) = Pubkey::from_str(addr) {
            return Some(AccountConstraint::Fixed(pk));
        }
    }

    // PDA derivation
    if let Some(pda) = &acc.pda {
        return Some(AccountConstraint::Pda(map_pda(pda)));
    }

    // Signer-only accounts
    if acc.signer && acc.address.is_none() && acc.pda.is_none() {
        return Some(AccountConstraint::Signer);
    }

    None
}

fn classify_known_program(addr: &str) -> Option<KnownProgram> {
    match addr {
        SYSTEM_PROGRAM => Some(KnownProgram::System),
        TOKEN_PROGRAM => Some(KnownProgram::Token),
        TOKEN_2022_PROGRAM => Some(KnownProgram::Token2022),
        ASSOCIATED_TOKEN_PROGRAM => Some(KnownProgram::AssociatedToken),
        _ => None,
    }
}

fn map_pda(pda: &IdlPda) -> PdaRule {
    let program = match &pda.program {
        None => PdaProgram::Self_,
        Some(seed) => map_program_seed(seed),
    };
    let seeds = pda.seeds.iter().map(map_seed).collect();
    PdaRule {
        program,
        seeds,
        bump_field: None,
    }
}

fn map_program_seed(seed: &IdlSeed) -> PdaProgram {
    match seed {
        IdlSeed::Const(c) => {
            // Try to interpret as a pubkey
            if c.value.len() == 32 {
                if let Ok(pk) = Pubkey::try_from(c.value.as_slice()) {
                    return match pk.to_string().as_str() {
                        SYSTEM_PROGRAM => PdaProgram::Known(KnownProgram::System),
                        TOKEN_PROGRAM => PdaProgram::Known(KnownProgram::Token),
                        TOKEN_2022_PROGRAM => PdaProgram::Known(KnownProgram::Token2022),
                        _ => PdaProgram::Self_,
                    };
                }
            }
            PdaProgram::Self_
        }
        IdlSeed::Account(a) => PdaProgram::AccountField(a.path.clone()),
        IdlSeed::Arg(a) => PdaProgram::AccountField(a.path.clone()),
    }
}

fn map_seed(seed: &IdlSeed) -> SeedComponent {
    match seed {
        IdlSeed::Const(c) => SeedComponent::Literal(c.value.clone()),
        IdlSeed::Arg(a) => SeedComponent::InstructionArg(a.path.clone()),
        IdlSeed::Account(a) => {
            // Anchor uses "account_name.field" notation.
            // If path contains a dot and ends with "key" it's the pubkey bytes.
            let path = &a.path;
            if path.ends_with(".key()") || path.ends_with(".key") {
                let name = path
                    .trim_end_matches(".key()")
                    .trim_end_matches(".key")
                    .to_string();
                SeedComponent::AccountKey(name)
            } else {
                SeedComponent::AccountField(path.clone())
            }
        }
    }
}

fn map_field(field: IdlField) -> InstructionArg {
    InstructionArg {
        name: field.name,
        ty: map_type(field.ty),
    }
}

fn map_type(ty: AnchorType) -> IdlType {
    match ty {
        AnchorType::Bool => IdlType::Bool,
        AnchorType::U8 => IdlType::U8,
        AnchorType::U16 => IdlType::U16,
        AnchorType::U32 => IdlType::U32,
        AnchorType::U64 => IdlType::U64,
        AnchorType::U128 => IdlType::U128,
        AnchorType::U256 => IdlType::U256,
        AnchorType::I8 => IdlType::I8,
        AnchorType::I16 => IdlType::I16,
        AnchorType::I32 => IdlType::I32,
        AnchorType::I64 => IdlType::I64,
        AnchorType::I128 => IdlType::I128,
        AnchorType::I256 => IdlType::I256,
        AnchorType::F32 => IdlType::F32,
        AnchorType::F64 => IdlType::F64,
        AnchorType::Bytes => IdlType::Bytes,
        AnchorType::String => IdlType::String,
        AnchorType::Pubkey => IdlType::PublicKey,
        AnchorType::Option(inner) => IdlType::Option(Box::new(map_type(*inner))),
        AnchorType::Vec(inner) => IdlType::Vec(Box::new(map_type(*inner))),
        AnchorType::Array(inner, len) => {
            let n = match len {
                anchor_lang_idl_spec::IdlArrayLen::Value(n) => n as u64,
                anchor_lang_idl_spec::IdlArrayLen::Generic(_) => 0,
            };
            IdlType::Array(Box::new(map_type(*inner)), n)
        }
        AnchorType::Defined { name, .. } => IdlType::Struct(name),
        AnchorType::Generic(name) => IdlType::Struct(name),
        _ => IdlType::Bytes, // fallback for future spec variants
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn escrow_idl_json() -> &'static str {
        r#"
{
  "address": "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
  "metadata": {
    "name": "escrow",
    "version": "0.1.0",
    "spec": "0.1.0"
  },
  "instructions": [
    {
      "name": "initialize",
      "discriminator": [175, 175, 109, 31, 13, 152, 155, 237],
      "accounts": [
        {
          "name": "initializer",
          "writable": true,
          "signer": true
        },
        {
          "name": "escrowState",
          "writable": true,
          "pda": {
            "seeds": [
              { "kind": "const", "value": [101, 115, 99, 114, 111, 119] },
              { "kind": "account", "path": "initializer.key" }
            ]
          }
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        { "name": "amount", "type": "u64" },
        { "name": "receiver", "type": "pubkey" }
      ]
    },
    {
      "name": "release",
      "discriminator": [111, 100, 43, 210, 75, 30, 102, 55],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "escrowState",
          "writable": true
        },
        {
          "name": "recipient",
          "writable": true
        }
      ],
      "args": []
    }
  ]
}
        "#
    }

    #[test]
    fn parses_program_id() {
        let idl = parse_anchor_idl_str(escrow_idl_json()).unwrap();
        assert_eq!(
            idl.program_id.to_string(),
            "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
        );
    }

    #[test]
    fn parses_instruction_count_and_names() {
        let idl = parse_anchor_idl_str(escrow_idl_json()).unwrap();
        assert_eq!(idl.instructions.len(), 2);
        assert_eq!(idl.instructions[0].name, "initialize");
        assert_eq!(idl.instructions[1].name, "release");
    }

    #[test]
    fn parses_discriminators() {
        let idl = parse_anchor_idl_str(escrow_idl_json()).unwrap();
        assert_eq!(
            idl.instructions[0].discriminator,
            vec![175, 175, 109, 31, 13, 152, 155, 237]
        );
    }

    #[test]
    fn maps_signer_constraint() {
        let idl = parse_anchor_idl_str(escrow_idl_json()).unwrap();
        let initializer = &idl.instructions[0].accounts[0];
        assert!(initializer.signer);
        assert!(initializer.writable);
        assert!(matches!(
            initializer.constraint,
            Some(AccountConstraint::Signer)
        ));
    }

    #[test]
    fn maps_pda_with_literal_and_key_seeds() {
        let idl = parse_anchor_idl_str(escrow_idl_json()).unwrap();
        let escrow_acc = &idl.instructions[0].accounts[1];
        match &escrow_acc.constraint {
            Some(AccountConstraint::Pda(rule)) => {
                assert!(matches!(rule.program, PdaProgram::Self_));
                assert_eq!(rule.seeds.len(), 2);
                assert!(matches!(&rule.seeds[0], SeedComponent::Literal(b) if b == b"escrow"));
                assert!(matches!(&rule.seeds[1], SeedComponent::AccountKey(n) if n == "initializer"));
            }
            other => panic!("expected PDA constraint, got {:?}", other),
        }
    }

    #[test]
    fn maps_system_program_constraint() {
        let idl = parse_anchor_idl_str(escrow_idl_json()).unwrap();
        let sys = &idl.instructions[0].accounts[2];
        assert!(matches!(
            sys.constraint,
            Some(AccountConstraint::Program(KnownProgram::System))
        ));
    }

    #[test]
    fn maps_instruction_args() {
        let idl = parse_anchor_idl_str(escrow_idl_json()).unwrap();
        let args = &idl.instructions[0].args;
        assert_eq!(args.len(), 2);
        assert_eq!(args[0].name, "amount");
        assert!(matches!(args[0].ty, IdlType::U64));
        assert_eq!(args[1].name, "receiver");
        assert!(matches!(args[1].ty, IdlType::PublicKey));
    }
}
