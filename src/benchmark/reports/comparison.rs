//! Comparison utilities for FSE and baseline execution.

use super::ordering::sort_points_lexicographically;
use super::timing::{
    RepeatedComparisonTimingReport, RepeatedTimingConfig, TimingReport, duration_ratio,
    measure_elapsed, measure_repeated_comparison_interleaved,
};
use crate::benchmark::baselines::{
    BaselineComparisonLabels, BaselineQueryStats, FlatScanBaseline, RangeQueryBaseline,
};
use crate::math::{Scalar, Vector};
use crate::query::{
    QueryExecutionOptions, QueryExecutionStats, QueryRegion, execute_query_with_stats_and_options,
};
use crate::storage::FSEIndex;

/// Side-by-side report comparing FSE query execution with baseline execution.
///
/// # Runtime Role
///
/// `QueryComparisonReport` is intended for early correctness and performance
/// analysis. It compares logical execution work and lightweight elapsed timing
/// between a baseline query path and the FSE query path.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryComparisonReport {
    /// Human-readable labels for this comparison.
    pub labels: BaselineComparisonLabels,

    /// Human-readable baseline name.
    pub baseline_name: String,

    /// Statistics from the baseline execution path.
    pub baseline_stats: BaselineQueryStats,

    /// Statistics from the FSE execution path.
    pub fse_stats: QueryExecutionStats,

    /// Wall-clock timing measurements for one execution of both paths.
    pub timing: TimingReport,

    /// Repeated timing measurements for both execution paths.
    pub repeated_timing: RepeatedComparisonTimingReport,

    /// Single-run timing ratio computed as baseline elapsed divided by FSE elapsed.
    pub single_run_timing_ratio: f64,

    /// Average timing ratio computed as baseline average elapsed divided by FSE average elapsed.
    pub average_timing_ratio: f64,

    /// Number of records avoided by FSE reconstruction relative to baseline evaluation.
    pub avoided_reconstructions: usize,

    /// Fraction of baseline record evaluations avoided by FSE reconstruction.
    pub reconstruction_avoidance_ratio: Scalar,

    /// Fraction of total records reconstructed by FSE.
    pub candidate_ratio: Scalar,

    /// Fraction of leaf partitions retained by FSE.
    pub retained_leaf_ratio: Scalar,
}

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
    let (baseline_report, baseline_elapsed) = measure_elapsed(|| baseline.execute(query));
    let (fse_report, fse_elapsed) =
        measure_elapsed(|| execute_query_with_stats_and_options(index, query, fse_options));

    let mut baseline_results = baseline_report.results;
    let mut fse_results = fse_report.results;

    sort_points_lexicographically(&mut baseline_results);
    sort_points_lexicographically(&mut fse_results);

    assert_eq!(
        fse_results, baseline_results,
        "FSE query results must match baseline query results"
    );

    let repeated_timing = measure_repeated_comparison_interleaved(
        timing_config,
        || {
            let _ = baseline.execute(query);
        },
        || {
            let _ = execute_query_with_stats_and_options(index, query, fse_options);
        },
    );

    let single_run_timing_ratio = duration_ratio(baseline_elapsed, fse_elapsed);
    let average_timing_ratio = duration_ratio(
        repeated_timing.baseline.average_elapsed,
        repeated_timing.fse.average_elapsed,
    );

    let evaluated_records = baseline_report.stats.evaluated_records;
    let reconstructed_records = fse_report.stats.reconstructed_records;

    let avoided_reconstructions = evaluated_records.saturating_sub(reconstructed_records);

    let reconstruction_avoidance_ratio = if evaluated_records == 0 {
        0.0
    } else {
        avoided_reconstructions as Scalar / evaluated_records as Scalar
    };

    let candidate_ratio = fse_report.stats.candidate_ratio;
    let retained_leaf_ratio = fse_report.stats.retained_leaf_ratio;
    let labels = baseline.labels();

    QueryComparisonReport {
        labels,
        baseline_name: baseline_report.baseline_name,
        baseline_stats: baseline_report.stats,
        fse_stats: fse_report.stats,
        timing: TimingReport {
            baseline_elapsed,
            fse_elapsed,
        },
        repeated_timing,
        single_run_timing_ratio,
        average_timing_ratio,
        avoided_reconstructions,
        reconstruction_avoidance_ratio,
        candidate_ratio,
        retained_leaf_ratio,
    }
}
