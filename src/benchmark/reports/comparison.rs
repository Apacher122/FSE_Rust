//! Comparison utilities for FSE and baseline execution.

use super::ordering::sort_points_lexicographically;
use super::timing::{
    RepeatedComparisonTimingReport, RepeatedTimingConfig, RepeatedTimingReport, TimingReport,
    duration_ratio, measure_elapsed, measure_repeated, measure_repeated_comparison_interleaved,
};
use crate::benchmark::baselines::{
    BaselineComparisonLabels, BaselineQueryStats, FlatScanBaseline, RangeQueryBaseline,
};
use crate::math::{Scalar, Vector};
use crate::query::{
    QueryExecutionOptions, QueryExecutionStats, QueryRegion, count_query_matches_with_stats,
    execute_query_into_with_options, execute_query_references_with_stats,
    execute_query_with_stats_and_options,
};
use crate::storage::FSEIndex;
use std::time::Duration;

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

    /// Statistics from the FSE owned-result execution path.
    pub fse_stats: QueryExecutionStats,

    /// Statistics from the FSE count-only execution path.
    pub count_only_stats: QueryExecutionStats,

    /// Statistics from the FSE reference-result execution path.
    pub reference_stats: QueryExecutionStats,

    /// Statistics from the FSE reusable owned-result execution path.
    pub reusable_owned_stats: QueryExecutionStats,

    /// Wall-clock timing measurements for one execution of both paths.
    pub timing: TimingReport,

    /// Repeated timing measurements for baseline and owned-result FSE execution.
    pub repeated_timing: RepeatedComparisonTimingReport,

    /// Repeated timing measurements for count-only FSE execution.
    pub count_only_repeated_timing: RepeatedTimingReport,

    /// Repeated timing measurements for reference-result FSE execution.
    pub reference_repeated_timing: RepeatedTimingReport,

    /// Repeated timing measurements for reusable owned-result FSE execution.
    pub reusable_owned_repeated_timing: RepeatedTimingReport,

    /// Estimated average elapsed time spent above count-only execution.
    pub estimated_owned_result_overhead: Duration,

    /// Estimated average elapsed time spent above reference-result execution.
    pub estimated_owned_vs_reference_overhead: Duration,

    /// Estimated average elapsed time spent above reusable owned-result execution.
    pub estimated_fresh_vs_reusable_owned_overhead: Duration,

    /// Average owned-result FSE elapsed divided by average count-only FSE elapsed.
    pub count_only_speedup_ratio: f64,

    /// Average owned-result FSE elapsed divided by average reference-result FSE elapsed.
    pub reference_result_speedup_ratio: f64,

    /// Average fresh owned-result FSE elapsed divided by average reusable owned-result FSE elapsed.
    pub reusable_owned_result_speedup_ratio: f64,

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
    let count_report = count_query_matches_with_stats(index, query);
    let reference_report = execute_query_references_with_stats(index, query);
    let mut reusable_owned_results = Vec::new();
    let reusable_owned_stats =
        execute_query_into_with_options(index, query, fse_options, &mut reusable_owned_results);

    assert_eq!(
        count_report.matched_records, fse_report.stats.matched_records,
        "count-only FSE query count must match owned-result FSE query count"
    );

    assert_eq!(
        count_report.stats, fse_report.stats,
        "count-only FSE structural stats must match owned-result FSE structural stats"
    );

    assert_eq!(
        reference_report.matches.len(),
        count_report.matched_records,
        "reference-result FSE query count must match count-only FSE query count"
    );

    assert_eq!(
        reference_report.stats, count_report.stats,
        "reference-result FSE structural stats must match count-only FSE structural stats"
    );

    assert_eq!(
        reusable_owned_stats, fse_report.stats,
        "reusable owned-result FSE structural stats must match fresh owned-result stats"
    );

    let mut baseline_results = baseline_report.results;
    let mut fse_results = fse_report.results;

    sort_points_lexicographically(&mut baseline_results);
    sort_points_lexicographically(&mut fse_results);
    sort_points_lexicographically(&mut reusable_owned_results);

    assert_eq!(
        fse_results, baseline_results,
        "FSE query results must match baseline query results"
    );

    assert_eq!(
        reusable_owned_results, baseline_results,
        "reusable owned-result FSE query results must match baseline query results"
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

    let count_only_repeated_timing = measure_repeated(timing_config, || {
        let report = count_query_matches_with_stats(index, query);
        std::hint::black_box(report.matched_records);
    });

    let reference_repeated_timing = measure_repeated(timing_config, || {
        let report = execute_query_references_with_stats(index, query);
        std::hint::black_box(report.matches.len());
    });

    let mut reusable_timing_results = Vec::new();

    let reusable_owned_repeated_timing = measure_repeated(timing_config, || {
        let stats = execute_query_into_with_options(
            index,
            query,
            fse_options,
            &mut reusable_timing_results,
        );

        std::hint::black_box(stats.matched_records);
        std::hint::black_box(reusable_timing_results.len());
    });

    let estimated_owned_result_overhead = repeated_timing
        .fse
        .average_elapsed
        .saturating_sub(count_only_repeated_timing.average_elapsed);
    let estimated_owned_vs_reference_overhead = repeated_timing
        .fse
        .average_elapsed
        .saturating_sub(reference_repeated_timing.average_elapsed);
    let estimated_fresh_vs_reusable_owned_overhead = repeated_timing
        .fse
        .average_elapsed
        .saturating_sub(reusable_owned_repeated_timing.average_elapsed);
    let count_only_speedup_ratio = duration_ratio(
        repeated_timing.fse.average_elapsed,
        count_only_repeated_timing.average_elapsed,
    );
    let reference_result_speedup_ratio = duration_ratio(
        repeated_timing.fse.average_elapsed,
        reference_repeated_timing.average_elapsed,
    );
    let reusable_owned_result_speedup_ratio = duration_ratio(
        repeated_timing.fse.average_elapsed,
        reusable_owned_repeated_timing.average_elapsed,
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
        count_only_stats: count_report.stats,
        reference_stats: reference_report.stats,
        reusable_owned_stats,
        timing: TimingReport {
            baseline_elapsed,
            fse_elapsed,
        },
        repeated_timing,
        count_only_repeated_timing,
        reference_repeated_timing,
        reusable_owned_repeated_timing,
        estimated_owned_result_overhead,
        estimated_owned_vs_reference_overhead,
        estimated_fresh_vs_reusable_owned_overhead,
        count_only_speedup_ratio,
        reference_result_speedup_ratio,
        reusable_owned_result_speedup_ratio,
        single_run_timing_ratio,
        average_timing_ratio,
        avoided_reconstructions,
        reconstruction_avoidance_ratio,
        candidate_ratio,
        retained_leaf_ratio,
    }
}
