//! `Ctx` — the user-facing fuzzing context.
use std::sync::{Arc, Mutex};

use praxis_core::{ExecResult, NormalIdl, NormalInstruction, Svm};
use praxis_gen::{AccountEntry, AccountSet, MutationStrategy};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer as _};

use crate::{
    engine::{fuzz_instruction, replay as engine_replay, Invariant, RunConfig},
    error::FuzzError,
    finding::Finding,
};

/// User-facing fuzzing context.  Build one via `Ctx::new()`, register
/// invariants, then call `fuzz_instructions()` or `run()`.
pub struct Ctx {
    svm: Arc<Mutex<Box<dyn Svm>>>,
    program_id: Pubkey,
    idl: Option<NormalIdl>,
    invariants: Vec<Invariant>,
    config: RunConfig,
}

impl Ctx {
    /// Create a new context wrapping `svm`.
    pub fn new(svm: Box<dyn Svm>, program_id: Pubkey) -> Self {
        Self {
            svm: Arc::new(Mutex::new(svm)),
            program_id,
            idl: None,
            invariants: Vec::new(),
            config: RunConfig::default(),
        }
    }

    // ── Builder methods ───────────────────────────────────────────────────────

    pub fn with_idl(mut self, idl: NormalIdl) -> Self {
        self.idl = Some(idl);
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.config.seed = seed;
        self
    }

    pub fn with_iterations(mut self, n: u32) -> Self {
        self.config.iterations = n;
        self
    }

    pub fn with_mutations(mut self, mutations: Vec<MutationStrategy>) -> Self {
        self.config.mutations = mutations;
        self
    }

    pub fn with_findings_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.config.findings_dir = dir.into();
        self
    }

    pub fn without_persistence(mut self) -> Self {
        self.config.persist_findings = false;
        self
    }

    // ── Account spawn helpers ─────────────────────────────────────────────────

    /// Spawn a fresh funded signer and load it into the SVM.
    /// Returns the keypair so the caller can sign transactions.
    pub fn spawn_signer(&self, seed: u64) -> Arc<Keypair> {
        use praxis_gen::rng::Rng;
        let mut rng = Rng::new(seed);
        let kp = Arc::new(
            Keypair::from_bytes(&rng.next_bytes::<64>())
                .unwrap_or_else(|_| Keypair::new()),
        );
        let acc = solana_sdk::account::Account {
            lamports: 1_000_000_000,
            ..Default::default()
        };
        let mut vm = self.svm.lock().unwrap();
        vm.set_account(&kp.pubkey(), acc);
        kp
    }

    /// Spawn a random funded account and load it into the SVM.
    pub fn spawn_account(&self, seed: u64) -> Pubkey {
        use praxis_gen::rng::Rng;
        let mut rng = Rng::new(seed);
        let pk = rng.next_pubkey();
        let acc = solana_sdk::account::Account {
            lamports: 1_000_000_000,
            ..Default::default()
        };
        let mut vm = self.svm.lock().unwrap();
        vm.set_account(&pk, acc);
        pk
    }

    /// Derive a PDA and return its address.
    pub fn spawn_pda(&self, seeds: &[&[u8]]) -> Pubkey {
        let (pda, _) = Pubkey::find_program_address(seeds, &self.program_id);
        pda
    }

    // ── Invariant registration ────────────────────────────────────────────────

    /// Register a named invariant predicate.
    ///
    /// The predicate receives the post-execution SVM state and `ExecResult`.
    /// Return `false` to signal a violation.
    pub fn invariant(
        &mut self,
        name: impl Into<String>,
        check: impl Fn(&dyn Svm, &ExecResult) -> bool + Send + Sync + 'static,
    ) {
        self.invariants.push(Invariant {
            name: name.into(),
            check: Arc::new(check),
        });
    }

    // ── Fuzzing entry points ──────────────────────────────────────────────────

    /// Fuzz a single instruction by name. Returns all findings.
    pub fn fuzz_instructions(&self, instruction_name: &str) -> Result<Vec<Finding>, FuzzError> {
        let idl = self.idl.as_ref().ok_or(FuzzError::NoIdl)?;
        let ix = find_instruction(idl, instruction_name)?;
        Ok(fuzz_instruction(
            Arc::clone(&self.svm),
            self.program_id,
            ix,
            &self.invariants,
            &self.config,
        ))
    }

    /// Fuzz every instruction in the IDL. Returns all findings across all instructions.
    pub fn run(&self) -> Result<Vec<Finding>, FuzzError> {
        let idl = self.idl.as_ref().ok_or(FuzzError::NoIdl)?;
        let mut all = Vec::new();
        for ix in &idl.instructions.clone() {
            let mut findings = fuzz_instruction(
                Arc::clone(&self.svm),
                self.program_id,
                ix,
                &self.invariants,
                &self.config,
            );
            all.append(&mut findings);
        }
        Ok(all)
    }

    /// Replay a finding by its ID (`<seed_hex>-<mutation_name>`).
    pub fn replay(&self, finding_id: &str) -> Result<Finding, FuzzError> {
        let (seed_hex, mutation_name) = finding_id.split_once('-').ok_or_else(|| {
            FuzzError::InvalidSeedHex(finding_id.to_owned(), "expected '<seed_hex>-<mutation>'".to_owned())
        })?;

        let seed = u64::from_str_radix(seed_hex, 16).map_err(|e| {
            FuzzError::InvalidSeedHex(seed_hex.to_owned(), e.to_string())
        })?;

        let idl = self.idl.as_ref().ok_or(FuzzError::NoIdl)?;

        // Try every instruction until one reproduces.
        for ix in &idl.instructions {
            if let Some(finding) = engine_replay(
                Arc::clone(&self.svm),
                self.program_id,
                ix,
                &self.invariants,
                seed,
                mutation_name,
            ) {
                return Ok(finding);
            }
        }

        Err(FuzzError::DidNotReproduce(finding_id.to_owned()))
    }

    // ── Direct SVM access ─────────────────────────────────────────────────────

    /// Load an `AccountSet` into the SVM (useful for test setup).
    pub fn load_accounts(&self, set: &AccountSet) {
        let mut vm = self.svm.lock().unwrap();
        for (_, entry) in &set.slots {
            vm.set_account(&entry.pubkey, entry.account.clone());
        }
    }

    /// Read an account from the SVM.
    pub fn account(&self, pk: &Pubkey) -> Option<solana_sdk::account::Account> {
        self.svm.lock().unwrap().account(pk)
    }
}

fn find_instruction<'a>(
    idl: &'a NormalIdl,
    name: &str,
) -> Result<&'a NormalInstruction, FuzzError> {
    idl.instructions
        .iter()
        .find(|ix| ix.name == name)
        .ok_or_else(|| FuzzError::InstructionNotFound(name.to_owned()))
}
