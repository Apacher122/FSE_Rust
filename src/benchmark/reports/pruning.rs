//! Pruning efficiency reporting.

use crate::benchmark::QueryComparisonReport;
use crate::math::Scalar;

/// Pruning-focused interpretation of a query comparison report.
///
/// # Runtime Role
///
/// `PruningEfficiencyReport` converts raw execution counters into ratios that
/// describe how effectively FSE eliminated work before reconstruction.
///
/// # Formal Reference
///
/// These values provide an implementation-level proxy for geometric selectivity
/// in the staged FSE execution model.
#[derive(Clone, Debug, PartialEq)]
pub struct PruningEfficiencyReport {
    /// Number of records evaluated by the flat scan baseline.
    pub baseline_records: usize,

    /// Number of records reconstructed by FSE.
    pub reconstructed_records: usize,

    /// Fraction of records reconstructed by FSE.
    pub candidate_ratio: Scalar,

    /// Fraction of baseline records not reconstructed by FSE.
    pub record_pruning_efficiency: Scalar,

    /// Total number of leaf partitions in the index.
    pub total_leaves: usize,

    /// Number of leaf partitions retained by FSE.
    pub retained_leaves: usize,

    /// Fraction of leaf partitions retained by FSE.
    pub retained_leaf_ratio: Scalar,

    /// Fraction of leaf partitions pruned by FSE.
    pub leaf_pruning_efficiency: Scalar,
}

/// Builds a pruning efficiency report from a query comparison report.
pub fn pruning_efficiency_report(comparison: &QueryComparisonReport) -> PruningEfficiencyReport {
    let candidate_ratio = comparison.candidate_ratio;
    let retained_leaf_ratio = comparison.retained_leaf_ratio;

    PruningEfficiencyReport {
        baseline_records: comparison.baseline_stats.evaluated_records,
        reconstructed_records: comparison.fse_stats.reconstructed_records,
        candidate_ratio,
        record_pruning_efficiency: complement_ratio(candidate_ratio),
        total_leaves: comparison.fse_stats.total_leaves,
        retained_leaves: comparison.fse_stats.retained_leaves,
        retained_leaf_ratio,
        leaf_pruning_efficiency: complement_ratio(retained_leaf_ratio),
    }
}

fn complement_ratio(value: Scalar) -> Scalar {
    1.0 - value
}
