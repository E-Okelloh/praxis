//! Core fuzz loop using proptest for generation and shrinking.
use std::sync::{Arc, Mutex};

use praxis_core::{ExecResult, NormalInstruction, Svm, SvmSnapshot};
use praxis_gen::{AccountSpawner, MutationStrategy, TxComposer};
use proptest::{
    prelude::*,
    test_runner::{Config as PropConfig, TestCaseError, TestRunner},
};
use solana_sdk::hash::Hash;

use crate::finding::Finding;

/// Parameters controlling a single fuzz run.
pub struct RunConfig {
    pub seed: u64,
    pub iterations: u32,
    pub mutations: Vec<MutationStrategy>,
    pub findings_dir: std::path::PathBuf,
    pub persist_findings: bool,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            seed: 0xDEAD_BEEF,
            iterations: 1_000,
            mutations: MutationStrategy::all().to_vec(),
            findings_dir: std::path::PathBuf::from(".praxis/findings"),
            persist_findings: true,
        }
    }
}

/// Type alias for the invariant predicate closure.
pub type InvariantFn = Arc<dyn Fn(&dyn Svm, &ExecResult) -> bool + Send + Sync>;

/// A registered invariant: name + predicate over post-execution state.
pub struct Invariant {
    pub name: String,
    pub check: InvariantFn,
}

/// Run the fuzzer for one instruction, returning all findings.
///
/// The SVM is wrapped in `Arc<Mutex<...>>` so the proptest `Fn` closure can
/// mutate it via interior mutability (single-threaded, no actual contention).
pub fn fuzz_instruction(
    svm: Arc<Mutex<Box<dyn Svm>>>,
    program_id: solana_sdk::pubkey::Pubkey,
    ix: &NormalInstruction,
    invariants: &[Invariant],
    config: &RunConfig,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Snapshot the clean pre-fuzz state once.
    let clean_snap: Arc<SvmSnapshot> = {
        let vm = svm.lock().unwrap();
        Arc::new(vm.snapshot())
    };

    let mutations = &config.mutations;
    let n_mutations = mutations.len();
    if n_mutations == 0 {
        return findings;
    }

    // Strategy: (iter_seed: u64, mutation_idx: usize)
    let strategy = (any::<u64>(), 0..n_mutations);

    let prop_config = PropConfig {
        cases: config.iterations,
        failure_persistence: None, // We handle persistence ourselves.
        ..PropConfig::default()
    };

    // Clone all the Arcs for capture by the Fn closure.
    let svm_c = Arc::clone(&svm);
    let snap_c = Arc::clone(&clean_snap);
    let ix_c = ix.clone();
    let program_id_c = program_id;
    let mutations_c = mutations.clone();
    let invariants_c: Vec<&Invariant> = invariants.iter().collect();

    // We collect findings through a Mutex inside the closure.
    let findings_acc: Arc<Mutex<Vec<Finding>>> = Arc::new(Mutex::new(Vec::new()));
    let findings_acc_c = Arc::clone(&findings_acc);

    // Seed proptest's RNG deterministically from our config seed.
    let rng_seed: Vec<u8> = config.seed.to_le_bytes().iter().cycle().take(32).cloned().collect();
    let rng = proptest::test_runner::TestRng::from_seed(
        proptest::test_runner::RngAlgorithm::ChaCha,
        &rng_seed,
    );
    let mut runner = TestRunner::new_with_rng(prop_config, rng);

    let _ = runner.run(&strategy, move |(iter_seed, mutation_idx)| {
        let mutation = mutations_c[mutation_idx];
        let mut vm = svm_c.lock().unwrap();

        // Restore clean state before each iteration.
        vm.restore(&snap_c);

        // Generate base accounts and apply mutation.
        let base_accounts = AccountSpawner::spawn(&ix_c, program_id_c, iter_seed);
        let mutated = mutation.apply(&ix_c, base_accounts, iter_seed.wrapping_add(0x1234));

        // Load all generated accounts into the SVM.
        for (_, entry) in &mutated.slots {
            vm.set_account(&entry.pubkey, entry.account.clone());
        }

        // Build and execute — use Hash::default(); LiteSVM callers should
        // construct their backend with blockhash checking disabled for fuzzing.
        let tx = TxComposer::compose(program_id_c, &ix_c, &mutated, &[], Hash::default());
        let result = vm.execute(tx);

        // Check every invariant.
        for inv in &invariants_c {
            if !(inv.check)(vm.as_ref(), &result) {
                let finding = Finding::new(
                    iter_seed,
                    mutation.name(),
                    &ix_c.name,
                    &inv.name,
                    result.logs.clone(),
                    result.cu_consumed,
                );
                findings_acc_c.lock().unwrap().push(finding);
                return Err(TestCaseError::fail(format!(
                    "invariant '{}' violated (seed={:#018x}, mutation={})",
                    inv.name, iter_seed, mutation.name()
                )));
            }
        }

        Ok(())
    });

    findings = findings_acc.lock().unwrap().clone();

    // Persist findings if requested.
    if config.persist_findings {
        for finding in &findings {
            let _ = finding.persist(&config.findings_dir);
        }
    }

    findings
}

/// Re-run a single (seed, mutation_name) pair and verify it still fails.
pub fn replay(
    svm: Arc<Mutex<Box<dyn Svm>>>,
    program_id: solana_sdk::pubkey::Pubkey,
    ix: &NormalInstruction,
    invariants: &[Invariant],
    seed: u64,
    mutation_name: &str,
) -> Option<Finding> {
    let mutation = MutationStrategy::all()
        .iter()
        .find(|m| m.name() == mutation_name)
        .copied()?;

    let clean_snap = {
        let vm = svm.lock().unwrap();
        vm.snapshot()
    };

    let mut vm = svm.lock().unwrap();
    vm.restore(&clean_snap);

    let base = AccountSpawner::spawn(ix, program_id, seed);
    let mutated = mutation.apply(ix, base, seed.wrapping_add(0x1234));

    for (_, entry) in &mutated.slots {
        vm.set_account(&entry.pubkey, entry.account.clone());
    }

    let tx = TxComposer::compose(program_id, ix, &mutated, &[], Hash::default());
    let result = vm.execute(tx);

    for inv in invariants {
        if !(inv.check)(vm.as_ref(), &result) {
            return Some(Finding::new(
                seed,
                mutation.name(),
                &ix.name,
                &inv.name,
                result.logs.clone(),
                result.cu_consumed,
            ));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use praxis_core::{AccountConstraint, ExecResult, IxAccountMeta, MockSvm, NormalInstruction};
    use solana_sdk::pubkey::Pubkey;

    fn signer_ix() -> NormalInstruction {
        NormalInstruction {
            name: "transfer".into(),
            discriminator: vec![1, 2, 3, 4, 5, 6, 7, 8],
            accounts: vec![
                IxAccountMeta {
                    name: "from".into(),
                    writable: true,
                    signer: true,
                    optional: false,
                    constraint: Some(AccountConstraint::Signer),
                },
                IxAccountMeta {
                    name: "to".into(),
                    writable: true,
                    signer: false,
                    optional: false,
                    constraint: None,
                },
            ],
            args: vec![],
        }
    }

    fn always_ok_svm() -> Arc<Mutex<Box<dyn Svm>>> {
        Arc::new(Mutex::new(Box::new(MockSvm::always_ok())))
    }

    fn always_fail_svm() -> Arc<Mutex<Box<dyn Svm>>> {
        Arc::new(Mutex::new(Box::new(MockSvm::always_fail())))
    }

    #[test]
    fn no_findings_when_invariant_always_passes() {
        let svm = always_ok_svm();
        let ix = signer_ix();
        let invariants = vec![Invariant {
            name: "always_true".into(),
            check: Arc::new(|_svm, _res| true),
        }];
        let config = RunConfig {
            seed: 42,
            iterations: 50,
            persist_findings: false,
            ..RunConfig::default()
        };
        let findings = fuzz_instruction(svm, Pubkey::new_unique(), &ix, &invariants, &config);
        assert!(findings.is_empty(), "expected no findings but got {}", findings.len());
    }

    #[test]
    fn finds_violation_when_invariant_fires_on_failure() {
        let svm = always_fail_svm();
        let ix = signer_ix();
        // Invariant: execution must succeed.
        let invariants = vec![Invariant {
            name: "must_succeed".into(),
            check: Arc::new(|_svm, res: &ExecResult| res.success),
        }];
        let config = RunConfig {
            seed: 1,
            iterations: 10,
            persist_findings: false,
            ..RunConfig::default()
        };
        let findings = fuzz_instruction(svm, Pubkey::new_unique(), &ix, &invariants, &config);
        assert!(!findings.is_empty(), "expected at least one finding");
        assert_eq!(findings[0].invariant_name, "must_succeed");
        assert_eq!(findings[0].instruction, "transfer");
    }

    #[test]
    fn finding_id_encodes_seed_and_mutation() {
        let svm = always_fail_svm();
        let ix = signer_ix();
        let invariants = vec![Invariant {
            name: "always_fail".into(),
            check: Arc::new(|_, _| false),
        }];
        let config = RunConfig {
            seed: 0xDEAD,
            iterations: 5,
            persist_findings: false,
            ..RunConfig::default()
        };
        let findings = fuzz_instruction(svm, Pubkey::new_unique(), &ix, &invariants, &config);
        assert!(!findings.is_empty());
        let f = &findings[0];
        // ID format: "<16 hex digits>-<MutationName>"
        let parts: Vec<&str> = f.id.splitn(2, '-').collect();
        assert_eq!(parts.len(), 2, "id should contain a dash separator");
        assert!(u64::from_str_radix(parts[0], 16).is_ok(), "first part should be hex seed");
    }

    #[test]
    fn replay_reproduces_finding() {
        let svm = always_fail_svm();
        let ix = signer_ix();
        let invariants = vec![Invariant {
            name: "always_fail".into(),
            check: Arc::new(|_, _| false),
        }];
        let config = RunConfig {
            seed: 77,
            iterations: 5,
            persist_findings: false,
            ..RunConfig::default()
        };
        let findings = fuzz_instruction(Arc::clone(&svm), Pubkey::new_unique(), &ix, &invariants, &config);
        assert!(!findings.is_empty());
        let f = &findings[0];
        let replayed = replay(
            Arc::clone(&svm),
            Pubkey::new_unique(),
            &ix,
            &invariants,
            f.seed,
            &f.mutation,
        );
        assert!(replayed.is_some(), "replay should reproduce the finding");
    }
}
