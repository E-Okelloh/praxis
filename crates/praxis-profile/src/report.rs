//! Aggregated statistics report for a profiling session.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::Sample;

/// Per-instruction statistics aggregated across all recorded samples.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionSummary {
    /// Instruction label.
    pub label: String,
    /// Number of times this instruction was observed.
    pub count: u64,
    /// Total CU across all observations.
    pub total_cu: u64,
    /// Average CU per execution.
    pub avg_cu: u64,
    /// Minimum CU observed.
    pub min_cu: u64,
    /// Maximum CU observed.
    pub max_cu: u64,
    /// Percentage of total session CU consumed by this instruction.
    pub pct_of_total: f64,
}

/// Summary report for a complete profiling session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileReport {
    /// Session / program name.
    pub name: String,
    /// Total CU across all samples.
    pub total_cu: u64,
    /// Total number of instruction executions recorded.
    pub total_samples: u64,
    /// Per-instruction statistics, sorted by `total_cu` descending.
    pub instructions: Vec<InstructionSummary>,
}

pub(crate) fn build_report(name: &str, samples: &[Sample]) -> ProfileReport {
    let total_cu: u64 = samples.iter().map(|s| s.cu).sum();
    let total_samples = samples.len() as u64;

    // Aggregate per label.
    let mut map: HashMap<&str, (u64, u64, u64, u64)> = HashMap::new(); // (count, total, min, max)
    for s in samples {
        let entry = map.entry(s.label.as_str()).or_insert((0, 0, u64::MAX, 0));
        entry.0 += 1;
        entry.1 += s.cu;
        entry.2 = entry.2.min(s.cu);
        entry.3 = entry.3.max(s.cu);
    }

    let mut instructions: Vec<InstructionSummary> = map
        .into_iter()
        .map(|(label, (count, total, min_cu, max_cu))| InstructionSummary {
            label: label.to_owned(),
            count,
            total_cu: total,
            avg_cu: total / count,
            min_cu,
            max_cu,
            pct_of_total: if total_cu > 0 {
                (total as f64 / total_cu as f64) * 100.0
            } else {
                0.0
            },
        })
        .collect();

    instructions.sort_by_key(|i| std::cmp::Reverse(i.total_cu));

    ProfileReport {
        name: name.to_owned(),
        total_cu,
        total_samples,
        instructions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_aggregates_correctly() {
        let samples = vec![
            Sample { label: "transfer".into(), cu: 4_000 },
            Sample { label: "transfer".into(), cu: 3_000 },
            Sample { label: "init".into(),     cu: 1_000 },
        ];
        let report = build_report("test", &samples);
        assert_eq!(report.total_cu, 8_000);
        assert_eq!(report.total_samples, 3);

        let transfer = report.instructions.iter().find(|i| i.label == "transfer").unwrap();
        assert_eq!(transfer.count, 2);
        assert_eq!(transfer.total_cu, 7_000);
        assert_eq!(transfer.min_cu, 3_000);
        assert_eq!(transfer.max_cu, 4_000);
        assert!((transfer.pct_of_total - 87.5).abs() < 0.1);
    }
}
