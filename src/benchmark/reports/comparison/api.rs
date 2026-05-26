//! Public query comparison API wrappers.

use crate::benchmark::baselines::{FlatScanBaseline, RangeQueryBaseline};
use crate::math::Vector;
use crate::query::{QueryExecutionOptions, QueryRegion};
use crate::storage::FSEIndex;

use super::super::timing::RepeatedTimingConfig;
use super::execution::run_query_comparison_with_baseline_and_options;
use super::report::QueryComparisonReport;

/// Compares FSE query execution against the default flat scan baseline.
///
/// # Runtime Role
///
/// This function preserves the existing flat-scan comparison API while routing
/// through the baseline abstraction and default FSE execution options.
pub fn compare_query_execution(
    index: &FSEIndex,
    points: &[Vector],
    query: &QueryRegion,
) -> QueryComparisonReport {
    compare_query_execution_with_options(index, points, query, QueryExecutionOptions::default())
}

/// Compares FSE query execution against the default flat scan baseline using explicit FSE options.
pub fn compare_query_execution_with_options(
    index: &FSEIndex,
    points: &[Vector],
    query: &QueryRegion,
    fse_options: QueryExecutionOptions,
) -> QueryComparisonReport {
    let baseline = FlatScanBaseline::new(points);

    compare_query_execution_with_baseline_and_options(
        index,
        query,
        &baseline,
        &RepeatedTimingConfig::default(),
        fse_options,
    )
}

/// Compares FSE query execution against the default flat scan baseline with repeated timing.
pub fn compare_query_execution_repeated(
    index: &FSEIndex,
    points: &[Vector],
    query: &QueryRegion,
    timing_config: &RepeatedTimingConfig,
) -> QueryComparisonReport {
    compare_query_execution_repeated_with_options(
        index,
        points,
        query,
        timing_config,
        QueryExecutionOptions::default(),
    )
}

/// Compares FSE query execution against the default flat scan baseline with repeated timing and explicit options.
pub fn compare_query_execution_repeated_with_options(
    index: &FSEIndex,
    points: &[Vector],
    query: &QueryRegion,
    timing_config: &RepeatedTimingConfig,
    fse_options: QueryExecutionOptions,
) -> QueryComparisonReport {
    let baseline = FlatScanBaseline::new(points);

    compare_query_execution_with_baseline_and_options(
        index,
        query,
        &baseline,
        timing_config,
        fse_options,
    )
}

/// Compares FSE query execution against a supplied baseline.
///
/// # Runtime Role
///
/// This function is the extension point for exact range-query baselines such as
/// flat scan, KD-tree, and R-tree implementations. It uses default FSE query
/// execution options.
///
/// # Panics
///
/// Panics when the FSE result set differs from the baseline result set.
pub fn compare_query_execution_with_baseline(
    index: &FSEIndex,
    query: &QueryRegion,
    baseline: &dyn RangeQueryBaseline,
    timing_config: &RepeatedTimingConfig,
) -> QueryComparisonReport {
    compare_query_execution_with_baseline_and_options(
        index,
        query,
        baseline,
        timing_config,
        QueryExecutionOptions::default(),
    )
}

/// Compares FSE query execution against a supplied baseline using explicit FSE options.
///
/// # Runtime Role
///
/// This function lets the benchmark layer choose serial or parallel FSE query
/// execution while preserving the same baseline comparison and correctness
/// checks.
///
/// # Panics
///
/// Panics when the FSE result set differs from the baseline result set.
pub fn compare_query_execution_with_baseline_and_options(
    index: &FSEIndex,
    query: &QueryRegion,
    baseline: &dyn RangeQueryBaseline,
    timing_config: &RepeatedTimingConfig,
    fse_options: QueryExecutionOptions,
) -> QueryComparisonReport {
    run_query_comparison_with_baseline_and_options(
        index,
        query,
        baseline,
        timing_config,
        fse_options,
    )
}
