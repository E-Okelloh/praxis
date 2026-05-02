//! `Finding` — a persisted invariant violation with a deterministic replay command.
use serde::{Deserialize, Serialize};

/// A confirmed invariant violation produced by the fuzz loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Unique identifier: `<seed_hex>-<mutation>`.
    pub id: String,
    /// The seed that produced this finding (deterministic replay key).
    pub seed: u64,
    /// Name of the mutation strategy that triggered the violation.
    pub mutation: String,
    /// Name of the instruction that was fuzzed.
    pub instruction: String,
    /// Name of the invariant that fired.
    pub invariant_name: String,
    /// Transaction logs from the failing execution.
    pub logs: Vec<String>,
    /// CU consumed by the failing transaction.
    pub cu_consumed: u64,
    /// Ready-to-paste replay command.
    pub replay_cmd: String,
}

impl Finding {
    pub(crate) fn new(
        seed: u64,
        mutation: &str,
        instruction: &str,
        invariant_name: &str,
        logs: Vec<String>,
        cu_consumed: u64,
    ) -> Self {
        let id = format!("{:016x}-{}", seed, mutation);
        let replay_cmd = format!("praxis replay --seed {id}");
        Self {
            id,
            seed,
            mutation: mutation.to_owned(),
            instruction: instruction.to_owned(),
            invariant_name: invariant_name.to_owned(),
            logs,
            cu_consumed,
            replay_cmd,
        }
    }

    /// Persist to `<dir>/<id>.json`, creating `dir` if needed.
    pub fn persist(&self, dir: &std::path::Path) -> Result<std::path::PathBuf, std::io::Error> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(format!("{}.json", self.id));
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(&path, json)?;
        Ok(path)
    }
}
