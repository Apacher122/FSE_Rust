//! CSV output configuration.

/// Output paths for benchmark CSV exports.
///
/// # Runtime Role
///
/// `BenchmarkCsvOutputConfig` groups CSV output destinations so CLI parsing and
/// benchmark execution do not need to pass each export path as a separate loose
/// field.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BenchmarkCsvOutputConfig {
    /// Optional path for writing the aggregate summary CSV.
    pub summary_path: Option<String>,

    /// Optional path for writing per-workload CSV rows.
    pub workloads_path: Option<String>,

    /// Optional path for writing the low-selectivity tree-gap CSV.
    pub low_selectivity_gap_path: Option<String>,
}

impl BenchmarkCsvOutputConfig {
    /// Creates a CSV output configuration from optional paths.
    pub fn new(summary_path: Option<String>, workloads_path: Option<String>) -> Self {
        Self {
            summary_path,
            workloads_path,
            low_selectivity_gap_path: None,
        }
    }

    /// Returns whether no CSV output paths were configured.
    pub fn is_empty(&self) -> bool {
        self.summary_path.is_none()
            && self.workloads_path.is_none()
            && self.low_selectivity_gap_path.is_none()
    }

    /// Returns whether aggregate summary CSV output was configured.
    pub fn has_summary_output(&self) -> bool {
        self.summary_path.is_some()
    }

    /// Returns whether per-workload CSV output was configured.
    pub fn has_workload_output(&self) -> bool {
        self.workloads_path.is_some()
    }

    /// Returns whether low-selectivity tree-gap CSV output was configured.
    pub fn has_low_selectivity_gap_output(&self) -> bool {
        self.low_selectivity_gap_path.is_some()
    }

    /// Sets the aggregate summary CSV output path.
    pub fn set_summary_path(&mut self, path: String) {
        // last path wins this matches the cli flag behavior
        self.summary_path = Some(path);
    }

    /// Sets the per-workload CSV output path.
    pub fn set_workloads_path(&mut self, path: String) {
        // same deal here no merge behavior for repeated flags
        self.workloads_path = Some(path);
    }

    /// Sets the low-selectivity tree-gap CSV output path.
    pub fn set_low_selectivity_gap_path(&mut self, path: String) {
        // repeated flags use the same last-path-wins behavior
        self.low_selectivity_gap_path = Some(path);
    }
}
