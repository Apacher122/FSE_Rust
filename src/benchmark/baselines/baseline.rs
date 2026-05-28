//! Baseline query execution abstractions.

use super::scan::FlatScanStats;
use crate::math::Vector;
use crate::query::QueryRegion;

/// Baseline implementation selected for a benchmark run.
///
/// # Runtime Role
///
/// `BaselineKind` provides a stable configuration-level identifier for choosing
/// which baseline engine should be used during comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaselineKind {
    /// Full linear scan over every record.
    FlatScan,

    /// Exact KD-tree range-query baseline.
    KdTree,

    /// Exact R-tree range-query baseline.
    RTree,
}

/// Baselines that provide exact range-query semantics.
///
/// # Runtime Role
///
/// This list is the shared source of truth for benchmark paths that need every
/// exact baseline currently implemented by the crate.
pub const EXACT_RANGE_BASELINE_KINDS: [BaselineKind; 3] = [
    BaselineKind::FlatScan,
    BaselineKind::KdTree,
    BaselineKind::RTree,
];

impl BaselineKind {
    /// Returns the stable baseline identifier.
    pub fn name(&self) -> &'static str {
        match self {
            BaselineKind::FlatScan => "flat_scan",
            BaselineKind::KdTree => "kd_tree",
            BaselineKind::RTree => "r_tree",
        }
    }
}

impl Default for BaselineKind {
    fn default() -> Self {
        Self::FlatScan
    }
}

/// Named baseline selection used by benchmark configuration.
///
/// # Runtime Role
///
/// `BenchmarkBaselineSet` captures the user's selection intent separately from
/// the concrete baseline list executed by the runner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchmarkBaselineSet {
    /// Run one explicitly selected baseline.
    Single(BaselineKind),

    /// Run every exact range-query baseline currently implemented.
    AllExact,
}

impl BenchmarkBaselineSet {
    /// Returns the stable name for this baseline set.
    pub fn name(&self) -> &'static str {
        match self {
            BenchmarkBaselineSet::Single(baseline_kind) => baseline_kind.name(),
            BenchmarkBaselineSet::AllExact => "all_exact",
        }
    }

    /// Returns the baseline kinds selected by this set.
    pub fn selected_kinds(&self) -> Vec<BaselineKind> {
        match self {
            BenchmarkBaselineSet::Single(baseline_kind) => vec![*baseline_kind],
            BenchmarkBaselineSet::AllExact => exact_range_baseline_vec(),
        }
    }

    /// Returns the selected baseline names.
    pub fn selected_names(&self) -> Vec<&'static str> {
        baseline_kind_names(&self.selected_kinds())
    }

    /// Returns a comma-separated list of selected baseline names.
    pub fn selected_name_list(&self) -> String {
        // keep this formatting in one place so overview and csv stay boring
        baseline_kind_name_list(&self.selected_kinds())
    }

    /// Returns whether this set represents a multi-baseline run.
    pub fn is_multi_baseline(&self) -> bool {
        self.selected_kinds().len() > 1
    }
}

impl Default for BenchmarkBaselineSet {
    fn default() -> Self {
        Self::Single(BaselineKind::default())
    }
}

/// Returns every exact range-query baseline kind.
///
/// # Runtime Role
///
/// This function gives callers a borrowed view over the canonical exact
/// baseline list. Use this when callers only need to iterate.
pub fn exact_range_baseline_kinds() -> &'static [BaselineKind] {
    // this is the one spot to add exact haselines later
    &EXACT_RANGE_BASELINE_KINDS
}

/// Returns every exact range-query baseline kind as an owned vector.
///
/// # Runtime Role
///
/// This function is useful for configuration paths that need ownership of the
/// selected baseline list.
pub fn exact_range_baseline_vec() -> Vec<BaselineKind> {
    // clone here so callers can own the run list without touching the constant
    exact_range_baseline_kinds().to_vec()
}

/// Returns stable baseline names for a selected baseline list.
pub fn baseline_kind_names(baseline_kinds: &[BaselineKind]) -> Vec<&'static str> {
    baseline_kinds
        .iter()
        .map(BaselineKind::name)
        .collect::<Vec<&'static str>>()
}

/// Returns a comma-separated list of stable baseline names.
pub fn baseline_kind_name_list(baseline_kinds: &[BaselineKind]) -> String {
    baseline_kind_names(baseline_kinds).join(", ")
}

/// Returns whether a selected baseline list contains multiple baselines.
pub fn has_multiple_baselines(baseline_kinds: &[BaselineKind]) -> bool {
    baseline_kinds.len() > 1
}

/// Common statistics reported by exact range-query baselines.
///
/// # Runtime Role
///
/// `BaselineQueryStats` gives the benchmark layer a shared stats shape for
/// baseline engines. Additional baselines can extend their own internal metrics
/// while still reporting those common comparison fields.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BaselineQueryStats {
    /// Number of records evaluated by the baseline.
    pub evaluated_records: usize,

    /// Number of records returned by the baseline.
    pub matched_records: usize,
}

impl From<FlatScanStats> for BaselineQueryStats {
    fn from(stats: FlatScanStats) -> Self {
        Self {
            evaluated_records: stats.evaluated_records,
            matched_records: stats.matched_records,
        }
    }
}

/// Human-readable labels for a baseline comparison.
///
/// # Runtime Role
///
/// `BaselineComparisonLabels` separates stable internal baseline names from
/// display labels used in benchmark output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaselineComparisonLabels {
    /// Stable baseline identifier.
    pub baseline_name: String,

    /// Human-readable baseline label.
    pub baseline_label: String,

    /// Human-readable FSE label.
    pub fse_label: String,

    /// Human-readable comparison label.
    pub comparison_label: String,
}

impl BaselineComparisonLabels {
    /// Builds labels for a baseline compared against FSE.
    pub fn new(baseline_name: impl Into<String>) -> Self {
        let baseline_name = baseline_name.into();
        let baseline_label = display_label_for_baseline(&baseline_name);
        let fse_label = "FSE".to_string();
        let comparison_label = format!("{} vs {}", baseline_label, fse_label);

        Self {
            baseline_name,
            baseline_label,
            fse_label,
            comparison_label,
        }
    }
}

/// Result returned by a benchmark baseline.
#[derive(Clone, Debug, PartialEq)]
pub struct BaselineQueryReport {
    /// Human-readable baseline name.
    pub baseline_name: String,

    /// Exact query results returned by the baseline.
    pub results: Vec<Vector>,

    /// Common baseline statistics.
    pub stats: BaselineQueryStats,
}

/// Baseline interface for exact range-query comparisons.
///
/// # Runtime Role
///
/// `RangeQueryBaseline` lets the benchmark layer compare FSE against different
/// exact query engines without hardcoding each engine into the comparison path.
pub trait RangeQueryBaseline {
    /// Returns the stable baseline name.
    fn name(&self) -> &'static str;

    /// Returns display labels for this baseline comparison.
    fn labels(&self) -> BaselineComparisonLabels {
        BaselineComparisonLabels::new(self.name())
    }

    /// Executes the baseline query.
    fn execute(&self, query: &QueryRegion) -> BaselineQueryReport;
}

fn display_label_for_baseline(name: &str) -> String {
    match name {
        "flat_scan" => "Flat Scan".to_string(),
        "kd_tree" => "KD-Tree".to_string(),
        "r_tree" => "R-Tree".to_string(),
        other => other.replace('_', " "),
    }
}
