//! Bug-bounty regression corpus — static IDL-based checks.
//!
//! Each test builds a synthetic `NormalIdl` that models the bug class of a
//! historical exploit and asserts that the relevant Praxis check fires.

use praxis_checks::{
    check_ac_001, check_ac_002, check_fd_002_staleness, check_fd_003_confidence,
    check_t22_002, report_cpi_finding, report_t22_001_reentrant_cpi,
};
use praxis_core::{IxAccountMeta, NormalIdl, NormalInstruction};
use solana_sdk::pubkey::Pubkey;

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_idl(ix_name: &str, accounts: Vec<IxAccountMeta>) -> NormalIdl {
    NormalIdl {
        name: "synthetic".into(),
        version: "0.0.0".into(),
        program_id: Pubkey::new_unique(),
        instructions: vec![NormalInstruction {
            name: ix_name.into(),
            discriminator: vec![0u8; 8],
            accounts,
            args: vec![],
        }],
    }
}

fn unsigned_authority(name: &str) -> IxAccountMeta {
    IxAccountMeta {
        name: name.into(),
        writable: false,
        signer: false,
        optional: false,
        constraint: None,
    }
}

fn unconstrained_data(name: &str) -> IxAccountMeta {
    IxAccountMeta {
        name: name.into(),
        writable: true,
        signer: false,
        optional: false,
        constraint: None,
    }
}

// ── 1. Wormhole — MissingSigner on guardian set upgrade ───────────────────────

/// The Wormhole exploit (Feb 2022, $320M): `verify_signatures` accepted a
/// spoofed sysvar account because the guardian set upgrade did not require the
/// guardian set to sign.  AC-001 fires on any `authority`-named account
/// without a signer constraint.
#[test]
fn wormhole_missing_signer_ac_001() {
    let idl = make_idl("upgrade_guardian_set", vec![
        unsigned_authority("guardian_set_authority"),
    ]);
    let findings = check_ac_001(&idl);
    assert!(!findings.is_empty(), "AC-001 must fire for unsigned guardian_set_authority");
    assert_eq!(findings[0].check_id, "AC-001");
}

// ── 2. Cashio — WrongOwner on collateral account ─────────────────────────────

/// Cashio exploit (Mar 2022, $52M): the `print_cash` instruction accepted any
/// account as the collateral vault without checking its owner.  AC-002 fires
/// on unconstrained writable accounts.
#[test]
fn cashio_wrong_owner_ac_002() {
    let idl = make_idl("print_cash", vec![
        unconstrained_data("collateral_vault"),
        unconstrained_data("collateral_metadata"),
    ]);
    let findings = check_ac_002(&idl);
    assert!(!findings.is_empty(), "AC-002 must fire for unconstrained collateral_vault");
    assert!(findings.iter().any(|f| f.check_id == "AC-002"));
}

// ── 3. Mango — stale oracle price ────────────────────────────────────────────

/// Mango exploit (Oct 2022, $114M): the attacker manipulated the on-chain
/// oracle price.  FD-002 detects stale `last_update_slot`.
#[test]
fn mango_stale_oracle_fd_002() {
    // Oracle last updated at slot 100, current slot is 700, max staleness = 100.
    let finding = check_fd_002_staleness("mango_sol_oracle", 100, 700, 100);
    assert!(finding.is_some(), "FD-002 must fire for staleness > threshold");
    assert_eq!(finding.unwrap().check_id, "FD-002");
}

#[test]
fn mango_fresh_oracle_passes() {
    // Oracle updated 50 slots ago, within the 100-slot threshold.
    let finding = check_fd_002_staleness("mango_sol_oracle", 650, 700, 100);
    assert!(finding.is_none());
}

// ── 4. Solend — oracle confidence not checked ─────────────────────────────────

/// FD-003: Solend-style wide-confidence-interval exploit — attackers could
/// push prices with high uncertainty.
#[test]
fn solend_confidence_interval_fd_003() {
    // Confidence 30% of price — above 10% threshold.
    let finding = check_fd_003_confidence("solend_sol_oracle", 1_000_000, 300_000, 0.10);
    assert!(finding.is_some(), "FD-003 must fire when conf/price > threshold");
    assert_eq!(finding.unwrap().check_id, "FD-003");
}

// ── 5. Drift — missing signer on authority update ─────────────────────────────

#[test]
fn drift_missing_admin_signer_ac_001() {
    let idl = make_idl("update_admin", vec![
        unsigned_authority("admin"),
    ]);
    let findings = check_ac_001(&idl);
    assert!(!findings.is_empty(), "AC-001 must fire for unsigned admin account");
}

// ── 6. Loopscale — arbitrary CPI ─────────────────────────────────────────────

/// report_cpi_finding models what the runtime fuzzer would emit when it
/// observes a CPI to a non-whitelisted program.
#[test]
fn loopscale_arbitrary_cpi_reported() {
    let finding = report_cpi_finding("repay_loan", "AttackerProgram111111111111111111111111111");
    assert_eq!(finding.check_id, "CPI-001");
}

// ── 7. Escrow — wrong PDA seeds accepted ─────────────────────────────────────

/// The escrow example's bug 3: `cancel` accepts an escrow account at a wrong
/// PDA.  AC-002 fires because the escrow_state has no owner/PDA constraint.
#[test]
fn escrow_wrong_pda_ac_002() {
    let idl = make_idl("cancel", vec![
        unconstrained_data("escrow_state"),
    ]);
    let findings = check_ac_002(&idl);
    assert!(!findings.is_empty(), "AC-002 must fire on unconstrained escrow_state");
}

// ── 8. Token-2022 — ExtraAccountMeta seeds not validated ─────────────────────

#[test]
fn t22_extra_meta_seeds_t22_002() {
    let idl = make_idl("initialize_extra_account_meta_list", vec![
        IxAccountMeta {
            name: "extra_account_metas".into(),
            writable: true,
            signer: false,
            optional: false,
            constraint: None, // No PDA constraint — T22-002
        },
    ]);
    let findings = check_t22_002(&idl);
    assert!(!findings.is_empty(), "T22-002 must fire for unconstrained extra_account_metas");
    assert_eq!(findings[0].check_id, "T22-002");
}

// ── 9. Token-2022 — re-entrant CPI in Transfer Hook ──────────────────────────

#[test]
fn t22_reentrant_cpi_t22_001() {
    let finding = report_t22_001_reentrant_cpi(
        "token-2022-hook",
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
    );
    assert_eq!(finding.check_id, "T22-001");
}

// ── 10. AMM — missing mint validation in swap ────────────────────────────────

/// The amm-pinocchio example's planted bug: `swap` accepts any token account
/// without verifying `mint == pool.mint_a`.  AC-002 would fire on the
/// source token account having no owner/mint constraint.
#[test]
fn amm_missing_mint_validation_ac_002() {
    let idl = make_idl("swap", vec![
        unconstrained_data("source_token_account"), // No mint constraint
    ]);
    let findings = check_ac_002(&idl);
    assert!(!findings.is_empty(), "AC-002 must fire on unconstrained source_token_account");
}
