//! CU profiler — collects compute-unit samples and emits SVG flame graphs.
//!
//! # Usage
//!
//! ```no_run
//! use praxis_profile::{Profiler, Sample};
//!
//! let mut profiler = Profiler::new("my_program");
//! profiler.record(Sample { label: "initialize".into(), cu: 1_200 });
//! profiler.record(Sample { label: "transfer".into(),   cu: 3_800 });
//!
//! let svg = profiler.flame_graph_svg().unwrap();
//! std::fs::write("profile.svg", svg).unwrap();
//! ```
#![deny(unsafe_code)]
#![warn(missing_docs)]

mod diff;
mod error;
mod flame;
mod report;

pub use diff::{CuDelta, ProfileDiff};
pub use error::ProfileError;
pub use flame::FlameConfig;
pub use report::{InstructionSummary, ProfileReport};

use serde::{Deserialize, Serialize};

/// A single compute-unit observation for one instruction execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    /// Human-readable label — typically the instruction name.
    pub label: String,
    /// Compute units consumed.
    pub cu: u64,
}

/// Collects `Sample`s and produces profiling artifacts.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Profiler {
    /// Name of the program or profiling session.
    pub name: String,
    samples: Vec<Sample>,
}

impl Profiler {
    /// Create a new profiler for a named program or session.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            samples: Vec::new(),
        }
    }

    /// Record one CU observation.
    pub fn record(&mut self, sample: Sample) {
        self.samples.push(sample);
    }

    /// All recorded samples in insertion order.
    pub fn samples(&self) -> &[Sample] {
        &self.samples
    }

    /// Total compute units across all recorded samples.
    pub fn total_cu(&self) -> u64 {
        self.samples.iter().map(|s| s.cu).sum()
    }

    /// Aggregate statistics grouped by instruction label.
    pub fn report(&self) -> ProfileReport {
        report::build_report(&self.name, &self.samples)
    }

    /// Render an SVG flame graph from recorded samples.
    pub fn flame_graph_svg(&self) -> Result<Vec<u8>, ProfileError> {
        flame::render_svg(&self.name, &self.samples, &FlameConfig::default())
    }

    /// Render an SVG flame graph with custom options.
    pub fn flame_graph_svg_with(&self, cfg: &FlameConfig) -> Result<Vec<u8>, ProfileError> {
        flame::render_svg(&self.name, &self.samples, cfg)
    }

    /// Compute the CU delta between `self` (new) and `baseline` (old).
    pub fn diff(&self, baseline: &Profiler) -> ProfileDiff {
        diff::compute_diff(baseline, self)
    }

    /// Serialize the profiler state to JSON for baseline storage.
    pub fn to_json(&self) -> Result<String, ProfileError> {
        serde_json::to_string_pretty(self).map_err(ProfileError::Json)
    }

    /// Deserialize from JSON (load a stored baseline).
    pub fn from_json(json: &str) -> Result<Self, ProfileError> {
        serde_json::from_str(json).map_err(ProfileError::Json)
    }
}
