//! Core query comparison execution.

use crate::benchmark::baselines::RangeQueryBaseline;
use crate::benchmark::math::scalar_ratio_or_zero;
use crate::benchmark::reports::ordering::sort_points_lexicographically;
use crate::benchmark::reports::timing::{
    RepeatedTimingConfig, TimingReport, duration_ratio, measure_elapsed, measure_repeated,
    measure_repeated_comparison_interleaved,
};
use crate::query::{
    QueryExecutionOptions, QueryRegion, count_query_matches_with_stats,
    execute_query_into_with_options, execute_query_references_with_stats,
    execute_query_with_stats_and_options,
};
use crate::storage::FSEIndex;

use super::report::QueryComparisonReport;

/// Runs the complete baseline/FSE comparison.
///
/// # Runtime Role
///
/// This is the shared implementation behind the public comparison helpers. It
/// performs single-run timing, repeated timing, exactness checks, count-only
/// comparison, reference-result comparison, and reusable owned-result
/// comparison before constructing the report consumed by benchmark summaries,
/// CSV export, and terminal output.
pub(super) fn run_query_comparison_with_baseline_and_options(
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

    let reconstruction_avoidance_ratio =
        scalar_ratio_or_zero(avoided_reconstructions, evaluated_records);

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
