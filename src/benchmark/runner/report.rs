//! Benchmark suite report types and construction helpers.

use crate::benchmark::baselines::BaselineKind;
use crate::benchmark::reports::{
    AggregateWorkloadMetrics, PruningEfficiencyReport, WorkloadComparisonSummary,
    aggregate_workload_metrics, pruning_efficiency_report,
};

/// Pruning report associated with a named workload.
#[derive(Clone, Debug, PartialEq)]
pub struct WorkloadPruningReport {
    /// Human-readable workload name.
    pub workload_name: String,

    /// Pruning-focused report for the workload.
    pub pruning: PruningEfficiencyReport,
}

/// Complete benchmark suite report for one selected baseline.
///
/// # Runtime Role
///
/// `BenchmarkSuiteReport` groups per-workload comparison summaries, aggregate
/// workload metrics, and pruning efficiency reports into one reusable output
/// object.
#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkSuiteReport {
    /// Per-workload FSE versus baseline comparison summaries.
    pub comparisons: Vec<WorkloadComparisonSummary>,

    /// Aggregate metrics across all workload comparisons.
    pub aggregate: AggregateWorkloadMetrics,

    /// Per-workload pruning efficiency reports.
    pub pruning_reports: Vec<WorkloadPruningReport>,
}

/// Benchmark suite report associated with one baseline kind.
#[derive(Clone, Debug, PartialEq)]
pub struct BaselineBenchmarkSuiteReport {
    /// Baseline kind used for this suite report.
    pub baseline_kind: BaselineKind,

    /// Stable baseline identifier.
    pub baseline_name: String,

    /// Benchmark report for this baseline.
    pub report: BenchmarkSuiteReport,
}

/// Complete benchmark report for multiple baselines.
///
/// # Runtime Role
///
/// `MultiBaselineBenchmarkSuiteReport` lets one benchmark run collect separate
/// FSE comparison reports for multiple baseline implementations.
#[derive(Clone, Debug, PartialEq)]
pub struct MultiBaselineBenchmarkSuiteReport {
    /// Per-baseline benchmark reports.
    pub baseline_reports: Vec<BaselineBenchmarkSuiteReport>,
}

pub(super) fn build_suite_report(
    comparisons: Vec<WorkloadComparisonSummary>,
) -> BenchmarkSuiteReport {
    let aggregate = aggregate_workload_metrics(&comparisons);

    let pruning_reports = comparisons
        .iter()
        .map(|summary| WorkloadPruningReport {
            workload_name: summary.workload_name.clone(),
            pruning: pruning_efficiency_report(&summary.comparison),
        })
        .collect();

    BenchmarkSuiteReport {
        comparisons,
        aggregate,
        pruning_reports,
    }
}
