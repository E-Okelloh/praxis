//! AC-001 and AC-002 — access-control checks.

use praxis_core::{AccountConstraint, NormalIdl};

use crate::types::{CheckFinding, Severity};

/// AC-001: Every account parameter whose name contains "authority", "owner",
/// or "admin" must have the `signer` flag set or an explicit `Signer` constraint.
///
/// This is a static check over the IDL.
pub fn check_ac_001(idl: &NormalIdl) -> Vec<CheckFinding> {
    let authority_patterns = ["authority", "owner", "admin", "manager", "controller"];
    let mut findings = Vec::new();

    for ix in &idl.instructions {
        for acc in &ix.accounts {
            let name_lower = acc.name.to_lowercase();
            let looks_like_authority = authority_patterns
                .iter()
                .any(|p| name_lower.contains(p));

            if !looks_like_authority {
                continue;
            }

            // It must either have `signer = true` or an explicit `Signer` constraint.
            let has_signer_flag = acc.signer;
            let has_signer_constraint = acc.constraint.as_ref().is_some_and(|c| {
                matches!(c, AccountConstraint::Signer)
            });

            if !has_signer_flag && !has_signer_constraint {
                findings.push(CheckFinding {
                    check_id: "AC-001".into(),
                    severity: Severity::High,
                    message: format!(
                        "Instruction `{}`: account `{}` looks like an authority but has no signer constraint",
                        ix.name, acc.name
                    ),
                    location: Some(format!("{}::{}", ix.name, acc.name)),
                });
            }
        }
    }

    findings
}

/// AC-002: Every non-system, non-sysvar account used in an instruction should
/// have an explicit owner constraint.  Accounts without one may accept arbitrary
/// program-owned data.
///
/// This is a static check over the IDL.
pub fn check_ac_002(idl: &NormalIdl) -> Vec<CheckFinding> {
    use praxis_core::AccountConstraint as AC;

    // These names are clearly not user-provided accounts and don't need owner checks.
    let exempt_patterns = [
        "system_program",
        "token_program",
        "rent",
        "clock",
        "associated_token_program",
        "payer",
        "fee_payer",
    ];

    let mut findings = Vec::new();

    for ix in &idl.instructions {
        for acc in &ix.accounts {
            let name_lower = acc.name.to_lowercase();
            if exempt_patterns.iter().any(|p| name_lower.contains(p)) {
                continue;
            }
            // Signer-only accounts that aren't deserialised don't need owner checks.
            if acc.signer && acc.constraint.is_none() {
                continue;
            }

            let has_owner_check = acc.constraint.as_ref().is_some_and(|c| {
                matches!(
                    c,
                    AC::Owner(_) | AC::Pda(_) | AC::Fixed(_) | AC::Program(_) | AC::Signer
                )
            });

            if !has_owner_check {
                findings.push(CheckFinding {
                    check_id: "AC-002".into(),
                    severity: Severity::Medium,
                    message: format!(
                        "Instruction `{}`: account `{}` has no explicit owner constraint — may accept arbitrary program-owned data",
                        ix.name, acc.name
                    ),
                    location: Some(format!("{}::{}", ix.name, acc.name)),
                });
            }
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use praxis_core::{IxAccountMeta, NormalIdl, NormalInstruction};
    use solana_sdk::pubkey::Pubkey;

    use super::*;

    fn make_idl(accounts: Vec<IxAccountMeta>) -> NormalIdl {
        NormalIdl {
            name: "test".into(),
            version: "0.1.0".into(),
            program_id: Pubkey::new_unique(),
            instructions: vec![NormalInstruction {
                name: "ix".into(),
                discriminator: vec![0; 8],
                accounts,
                args: vec![],
            }],
        }
    }

    #[test]
    fn ac_001_fires_on_unsigned_authority() {
        let idl = make_idl(vec![IxAccountMeta {
            name: "authority".into(),
            writable: false,
            signer: false,
            optional: false,
            constraint: None,
        }]);
        let findings = check_ac_001(&idl);
        assert!(!findings.is_empty(), "should flag unsigned authority");
        assert_eq!(findings[0].check_id, "AC-001");
    }

    #[test]
    fn ac_001_passes_when_signer_flag_set() {
        let idl = make_idl(vec![IxAccountMeta {
            name: "authority".into(),
            writable: false,
            signer: true,
            optional: false,
            constraint: None,
        }]);
        assert!(check_ac_001(&idl).is_empty());
    }

    #[test]
    fn ac_002_fires_on_unconstrained_data_account() {
        let idl = make_idl(vec![IxAccountMeta {
            name: "escrow_state".into(),
            writable: true,
            signer: false,
            optional: false,
            constraint: None,
        }]);
        let findings = check_ac_002(&idl);
        assert!(!findings.is_empty(), "should flag unconstrained data account");
        assert_eq!(findings[0].check_id, "AC-002");
    }

    #[test]
    fn ac_002_passes_on_owner_constrained_account() {
        let idl = make_idl(vec![IxAccountMeta {
            name: "escrow_state".into(),
            writable: true,
            signer: false,
            optional: false,
            constraint: Some(AccountConstraint::Owner(Pubkey::new_unique())),
        }]);
        assert!(check_ac_002(&idl).is_empty());
    }
}
