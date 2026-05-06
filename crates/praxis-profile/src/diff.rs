//! CU delta between two profiling sessions.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::Profiler;

/// Per-instruction CU delta between two profiling sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuDelta {
    /// Instruction label.
    pub label: String,
    /// Average CU in the baseline session (0 if absent).
    pub baseline_avg_cu: u64,
    /// Average CU in the new session (0 if absent).
    pub new_avg_cu: u64,
    /// Signed delta: `new_avg_cu` − `baseline_avg_cu`.
    pub delta: i64,
    /// Relative change as a percentage (positive = regression).
    pub pct_change: f64,
}

/// CU diff between a baseline and a new profiling session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDiff {
    /// Total CU in the baseline session.
    pub baseline_total_cu: u64,
    /// Total CU in the new session.
    pub new_total_cu: u64,
    /// Signed total delta.
    pub total_delta: i64,
    /// Per-instruction deltas, sorted by absolute delta descending.
    pub instructions: Vec<CuDelta>,
}

pub(crate) fn compute_diff(baseline: &Profiler, new: &Profiler) -> ProfileDiff {
    let baseline_report = baseline.report();
    let new_report = new.report();

    let baseline_map: HashMap<&str, u64> = baseline_report
        .instructions
        .iter()
        .map(|i| (i.label.as_str(), i.avg_cu))
        .collect();

    let new_map: HashMap<&str, u64> = new_report
        .instructions
        .iter()
        .map(|i| (i.label.as_str(), i.avg_cu))
        .collect();

    // Union of all labels.
    let mut all_labels: Vec<&str> = baseline_map
        .keys()
        .chain(new_map.keys())
        .copied()
        .collect();
    all_labels.sort_unstable();
    all_labels.dedup();

    let mut instructions: Vec<CuDelta> = all_labels
        .into_iter()
        .map(|label| {
            let b = *baseline_map.get(label).unwrap_or(&0);
            let n = *new_map.get(label).unwrap_or(&0);
            let delta = n as i64 - b as i64;
            let pct_change = if b > 0 {
                (delta as f64 / b as f64) * 100.0
            } else {
                f64::INFINITY
            };
            CuDelta {
                label: label.to_owned(),
                baseline_avg_cu: b,
                new_avg_cu: n,
                delta,
                pct_change,
            }
        })
        .collect();

    instructions.sort_by_key(|d| std::cmp::Reverse(d.delta.unsigned_abs()));

    ProfileDiff {
        baseline_total_cu: baseline_report.total_cu,
        new_total_cu: new_report.total_cu,
        total_delta: new_report.total_cu as i64 - baseline_report.total_cu as i64,
        instructions,
    }
}

#[cfg(test)]
mod tests {
    use crate::{Profiler, Sample};

    #[test]
    fn diff_detects_regression() {
        let mut baseline = Profiler::new("prog");
        baseline.record(Sample { label: "transfer".into(), cu: 3_000 });
        baseline.record(Sample { label: "init".into(),     cu: 1_000 });

        let mut new = Profiler::new("prog");
        new.record(Sample { label: "transfer".into(), cu: 4_000 }); // +1000
        new.record(Sample { label: "init".into(),     cu: 1_000 }); // unchanged

        let diff = new.diff(&baseline);
        assert_eq!(diff.total_delta, 1_000);

        let transfer_delta = diff
            .instructions
            .iter()
            .find(|d| d.label == "transfer")
            .unwrap();
        assert_eq!(transfer_delta.delta, 1_000);
        assert!((transfer_delta.pct_change - 33.333).abs() < 0.1);
    }
}
