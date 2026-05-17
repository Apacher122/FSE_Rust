//! Benchmark suite runner.

use crate::benchmark::baselines::{BaselineKind, BaselineRegistry};
use crate::benchmark::reports::{
    AggregateWorkloadMetrics, PruningEfficiencyReport, RepeatedTimingConfig,
    WorkloadComparisonSummary, aggregate_workload_metrics, compare_query_execution_repeated,
    compare_query_execution_repeated_with_options, compare_query_execution_with_baseline,
    compare_query_execution_with_baseline_and_options, pruning_efficiency_report,
};
use crate::benchmark::workloads::QueryWorkloadCase;
use crate::math::Vector;
use crate::query::QueryExecutionOptions;
use crate::storage::FSEIndex;

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

/// Runs the current FSE benchmark suite against a workload set.
///
/// # Runtime Role
///
/// This function centralizes benchmark orchestration so demos and future
/// benchmark harnesses do not need to manually wire together comparison,
/// aggregation, and pruning reports.
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

    build_suite_report(comparisons)
}

/// Runs the current FSE benchmark suite with repeated timing and explicit FSE execution options.
pub fn run_benchmark_suite_repeated_with_options(
    index: &FSEIndex,
    points: &[Vector],
    workloads: &[QueryWorkloadCase],
    timing_config: &RepeatedTimingConfig,
    fse_options: QueryExecutionOptions,
) -> BenchmarkSuiteReport {
    let comparisons: Vec<WorkloadComparisonSummary> = workloads
        .iter()
        .map(|workload| WorkloadComparisonSummary {
            workload_name: workload.name.clone(),
            comparison: compare_query_execution_repeated_with_options(
                index,
                points,
                &workload.query,
                timing_config,
                fse_options,
            ),
        })
        .collect();

    build_suite_report(comparisons)
}

/// Runs the benchmark suite with a configured baseline kind.
pub fn run_benchmark_suite_with_registry(
    index: &FSEIndex,
    points: &[Vector],
    workloads: &[QueryWorkloadCase],
    timing_config: &RepeatedTimingConfig,
    registry: &BaselineRegistry,
    baseline_kind: BaselineKind,
) -> BenchmarkSuiteReport {
    let baseline = registry.resolve(baseline_kind, points);

    let comparisons: Vec<WorkloadComparisonSummary> = workloads
        .iter()
        .map(|workload| WorkloadComparisonSummary {
            workload_name: workload.name.clone(),
            comparison: compare_query_execution_with_baseline(
                index,
                &workload.query,
                baseline.as_ref(),
                timing_config,
            ),
        })
        .collect();

    build_suite_report(comparisons)
}

/// Runs the benchmark suite with a configured baseline kind and explicit FSE execution options.
pub fn run_benchmark_suite_with_registry_and_options(
    index: &FSEIndex,
    points: &[Vector],
    workloads: &[QueryWorkloadCase],
    timing_config: &RepeatedTimingConfig,
    registry: &BaselineRegistry,
    baseline_kind: BaselineKind,
    fse_options: QueryExecutionOptions,
) -> BenchmarkSuiteReport {
    let baseline = registry.resolve(baseline_kind, points);

    let comparisons: Vec<WorkloadComparisonSummary> = workloads
        .iter()
        .map(|workload| WorkloadComparisonSummary {
            workload_name: workload.name.clone(),
            comparison: compare_query_execution_with_baseline_and_options(
                index,
                &workload.query,
                baseline.as_ref(),
                timing_config,
                fse_options,
            ),
        })
        .collect();

    build_suite_report(comparisons)
}

/// Runs the benchmark suite for multiple baseline kinds.
///
/// # Runtime Role
///
/// This function is used when the same FSE index and workload set should be
/// compared against several baseline implementations in one benchmark pass.
pub fn run_multi_baseline_benchmark_suite(
    index: &FSEIndex,
    points: &[Vector],
    workloads: &[QueryWorkloadCase],
    timing_config: &RepeatedTimingConfig,
    registry: &BaselineRegistry,
    baseline_kinds: &[BaselineKind],
) -> MultiBaselineBenchmarkSuiteReport {
    run_multi_baseline_benchmark_suite_with_options(
        index,
        points,
        workloads,
        timing_config,
        registry,
        baseline_kinds,
        QueryExecutionOptions::default(),
    )
}

/// Runs the benchmark suite for multiple baseline kinds using explicit FSE execution options.
pub fn run_multi_baseline_benchmark_suite_with_options(
    index: &FSEIndex,
    points: &[Vector],
    workloads: &[QueryWorkloadCase],
    timing_config: &RepeatedTimingConfig,
    registry: &BaselineRegistry,
    baseline_kinds: &[BaselineKind],
    fse_options: QueryExecutionOptions,
) -> MultiBaselineBenchmarkSuiteReport {
    let baseline_reports = baseline_kinds
        .iter()
        .map(|baseline_kind| {
            let report = run_benchmark_suite_with_registry_and_options(
                index,
                points,
                workloads,
                timing_config,
                registry,
                *baseline_kind,
                fse_options,
            );

            BaselineBenchmarkSuiteReport {
                baseline_kind: *baseline_kind,
                baseline_name: baseline_kind.name().to_string(),
                report,
            }
        })
        .collect();

    MultiBaselineBenchmarkSuiteReport { baseline_reports }
}

fn build_suite_report(comparisons: Vec<WorkloadComparisonSummary>) -> BenchmarkSuiteReport {
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
