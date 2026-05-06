//! T22-001, T22-002, T22-003 — Token-2022 / Transfer Hook checks.

use praxis_core::NormalIdl;

use crate::types::{CheckFinding, Severity};

/// T22-001: Transfer Hook must not CPI back into the same mint.
///
/// Re-entrancy via a Transfer Hook CPI into the same mint can allow double-spend
/// or bypass of balance checks.  This is a runtime check; the IDL cannot
/// statically detect it.
pub fn check_t22_001(_idl: &NormalIdl) -> Vec<CheckFinding> {
    vec![]
}

/// Report a T22-001 re-entrancy finding detected at runtime.
pub fn report_t22_001_reentrant_cpi(hook_program: &str, mint: &str) -> CheckFinding {
    CheckFinding {
        check_id: "T22-001".into(),
        severity: Severity::Critical,
        message: format!(
            "Transfer Hook `{hook_program}` issued a CPI back into mint `{mint}` — potential re-entrancy"
        ),
        location: Some(hook_program.to_owned()),
    }
}

/// T22-002: All `ExtraAccountMetaList` seeds must be validated.
///
/// Seeds that are not properly validated allow injection of malicious extra
/// accounts into the hook instruction.  This is a static + runtime check.
pub fn check_t22_002(idl: &NormalIdl) -> Vec<CheckFinding> {
    // Static heuristic: flag any instruction that accepts an "extra_account_meta"
    // or "extra_metas" account without a PDA constraint.
    let mut findings = Vec::new();
    let extra_patterns = ["extra_account_meta", "extra_metas", "extra_account"];

    for ix in &idl.instructions {
        for acc in &ix.accounts {
            let name_lower = acc.name.to_lowercase();
            if !extra_patterns.iter().any(|p| name_lower.contains(p)) {
                continue;
            }
            let has_pda = acc.constraint.as_ref().is_some_and(|c| {
                matches!(c, praxis_core::AccountConstraint::Pda(_))
            });
            if !has_pda {
                findings.push(CheckFinding {
                    check_id: "T22-002".into(),
                    severity: Severity::High,
                    message: format!(
                        "Instruction `{}`: `{}` looks like ExtraAccountMetaList but has no PDA seed constraint",
                        ix.name, acc.name
                    ),
                    location: Some(format!("{}::{}", ix.name, acc.name)),
                });
            }
        }
    }
    findings
}

/// T22-003: ZK proof inputs must match expected ciphertexts.
///
/// This is a runtime check requiring proof verification; the IDL cannot detect it.
pub fn check_t22_003(_idl: &NormalIdl) -> Vec<CheckFinding> {
    vec![]
}

/// Report a T22-003 ZK proof mismatch detected at runtime.
pub fn report_t22_003_zk_mismatch(ix_name: &str, detail: &str) -> CheckFinding {
    CheckFinding {
        check_id: "T22-003".into(),
        severity: Severity::Critical,
        message: format!("Instruction `{ix_name}` ZK proof input mismatch: {detail}"),
        location: Some(ix_name.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use praxis_core::{IxAccountMeta, NormalIdl, NormalInstruction};
    use solana_sdk::pubkey::Pubkey;

    use super::*;

    fn make_idl(accounts: Vec<IxAccountMeta>) -> NormalIdl {
        NormalIdl {
            name: "hook".into(),
            version: "0.1.0".into(),
            program_id: Pubkey::new_unique(),
            instructions: vec![NormalInstruction {
                name: "execute".into(),
                discriminator: vec![0; 8],
                accounts,
                args: vec![],
            }],
        }
    }

    #[test]
    fn t22_002_fires_on_unconstrained_extra_metas() {
        let idl = make_idl(vec![IxAccountMeta {
            name: "extra_account_metas".into(),
            writable: false,
            signer: false,
            optional: false,
            constraint: None,
        }]);
        let findings = check_t22_002(&idl);
        assert!(!findings.is_empty());
        assert_eq!(findings[0].check_id, "T22-002");
    }

    #[test]
    fn t22_002_passes_on_pda_constrained_metas() {
        use praxis_core::{AccountConstraint, PdaProgram, PdaRule};
        let idl = make_idl(vec![IxAccountMeta {
            name: "extra_account_metas".into(),
            writable: false,
            signer: false,
            optional: false,
            constraint: Some(AccountConstraint::Pda(PdaRule {
                program: PdaProgram::Self_,
                seeds: vec![],
                bump_field: None,
            })),
        }]);
        assert!(check_t22_002(&idl).is_empty());
    }
}
