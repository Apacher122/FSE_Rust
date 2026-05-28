//! Single-baseline benchmark suite execution.

use crate::benchmark::baselines::{BaselineKind, BaselineRegistry};
use crate::benchmark::reports::{
    RepeatedTimingConfig, WorkloadComparisonSummary, compare_query_execution_repeated,
    compare_query_execution_repeated_with_options, compare_query_execution_with_baseline,
    compare_query_execution_with_baseline_and_options,
};
use crate::benchmark::workloads::QueryWorkloadCase;
use crate::math::Vector;
use crate::query::QueryExecutionOptions;
use crate::storage::FSEIndex;

use super::report::{BenchmarkSuiteReport, build_suite_report};

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
