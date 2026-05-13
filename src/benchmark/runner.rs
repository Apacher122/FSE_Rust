//! Benchmark suite runner.

use crate::benchmark::{
    AggregateWorkloadMetrics, PruningEfficiencyReport, QueryWorkloadCase, RepeatedTimingConfig,
    WorkloadComparisonSummary, aggregate_workload_metrics, compare_query_execution_repeated,
    pruning_efficiency_report,
};
use crate::math::Vector;
use crate::storage::FSEIndex;

/// Pruning report associated with a named workload.
#[derive(Clone, Debug, PartialEq)]
pub struct WorkloadPruningReport {
    /// Human-readable workload name.
    pub workload_name: String,

    /// Pruning-focused report for the workload.
    pub pruning: PruningEfficiencyReport,
}

/// Complete benchmark suite report.
///
/// # Runtime Role
///
/// `BenchmarkSuiteReport` groups per-workload comparison summaries, aggregate
/// workload metrics, and pruning efficiency reports into one reusable output
/// object.
#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkSuiteReport {
    /// Per-workload FSE versus flat-scan comparison summaries.
    pub comparisons: Vec<WorkloadComparisonSummary>,

    /// Aggregate metrics across all workload comparisons.
    pub aggregate: AggregateWorkloadMetrics,

    /// Per-workload pruning efficiency reports.
    pub pruning_reports: Vec<WorkloadPruningReport>,
}

/// Runs the current FSE benchmark suite against a workload set.
///
/// # Runtime Role
///
/// This function centralizes benchmark orchestration so demos and future
/// benchmark harnesses do not need to manually wire together comparison,
/// aggregation, and pruning reports.
///
/// # Panics
///
/// Panics if any FSE query result differs from the flat scan result.
pub fn run_benchmark_suite(
    index: &FSEIndex,
    points: &[Vector],
    workloads: &[QueryWorkloadCase],
) -> BenchmarkSuiteReport {
    run_benchmark_suite_repeated(index, points, workloads, &RepeatedTimingConfig::default())
}

/// Runs the current FSE benchmark suite with repeated timing configuration.
///
/// # Runtime Role
///
/// This function lets callers tune the number of measured timing iterations
/// while reusing the same comparison and aggregation flow.
pub fn run_benchmark_suite_repeated(
    index: &FSEIndex,
    points: &[Vector],
    workloads: &[QueryWorkloadCase],
    timing_config: &RepeatedTimingConfig,
) -> BenchmarkSuiteReport {
    let comparisons: Vec<WorkloadComparisonSummary> = workloads
        .iter()
        .map(|workload| WorkloadComparisonSummary {
            workload_name: workload.name.clone(),
            comparison: compare_query_execution_repeated(
                index,
                points,
                &workload.query,
                timing_config,
            ),
        })
        .collect();

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
