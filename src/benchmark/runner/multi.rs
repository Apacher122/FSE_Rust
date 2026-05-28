//! Multi-baseline benchmark suite execution.

use crate::benchmark::baselines::{BaselineKind, BaselineRegistry};
use crate::benchmark::reports::RepeatedTimingConfig;
use crate::benchmark::workloads::QueryWorkloadCase;
use crate::math::Vector;
use crate::query::QueryExecutionOptions;
use crate::storage::FSEIndex;

use super::report::{BaselineBenchmarkSuiteReport, MultiBaselineBenchmarkSuiteReport};
use super::suite::run_benchmark_suite_with_registry_and_options;

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
