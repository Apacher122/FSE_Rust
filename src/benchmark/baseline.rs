//! Baseline query execution abstractions.

use crate::benchmark::{FlatScanStats, flat_scan_with_stats};
use crate::math::Vector;
use crate::query::QueryRegion;

/// Common statistics reported by exact range-query baselines.
///
/// # Runtime Role
///
/// `BaselineQueryStats` gives the benchmark layer a shared stats shape for
/// baseline engines. Additional baselines can extend their own internal metrics
/// while still reporting those common comparison fields.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BaselineQueryStats {
    /// number of records evaluated by the baseline
    pub evaluated_records: usize,

    /// number of records returned by the baseline
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

/// Result returned by a benchmark baseline
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
    /// Returns the stable baseline name
    fn name(&self) -> &'static str;

    /// Returns display labels for this baseline comparison.
    fn labels(&self) -> BaselineComparisonLabels {
        BaselineComparisonLabels::new(self.name())
    }

    /// Executes the baseline query.
    fn execute(&self, points: &[Vector], query: &QueryRegion) -> BaselineQueryReport;
}

/// Flat scan baseline implementation
///
/// # Runtime Role
///
/// `FlatScanBaseline` represents the conventional full-record evaluation path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlatScanBaseline;

impl RangeQueryBaseline for FlatScanBaseline {
    fn name(&self) -> &'static str {
        "flat_scan"
    }

    fn execute(&self, points: &[Vector], query: &QueryRegion) -> BaselineQueryReport {
        // Keep the first baseline intentionally simple: every record is evaluated.
        let report = flat_scan_with_stats(points, query);

        BaselineQueryReport {
            baseline_name: self.name().to_string(),
            results: report.results,
            stats: report.stats.into(),
        }
    }
}

/// Executes a range-query baseline.
///
/// # Runtime Role
///
/// This helper gives tests and benchmark runners a small common entrypoint for
/// baseline execution.
pub fn execute_range_baseline(
    baseline: &impl RangeQueryBaseline,
    points: &[Vector],
    query: &QueryRegion,
) -> BaselineQueryReport {
    baseline.execute(points, query)
}

fn display_label_for_baseline(name: &str) -> String {
    match name {
        "flat_scan" => "Flat Scan".to_string(),
        "kd_tree" => "KD-Tree".to_string(),
        "r_tree" => "R-Tree".to_string(),
        other => other.replace('_', " "),
    }
}
