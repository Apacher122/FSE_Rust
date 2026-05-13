//! Comparison utilities for FSE and flat scan execution.

use crate::benchmark::{
    BaselineComparisonLabels, BaselineQueryStats, FlatScanBaseline, RangeQueryBaseline,
    RepeatedComparisonTimingReport, RepeatedTimingConfig, TimingReport, duration_ratio,
    measure_elapsed, measure_repeated,
};

use crate::math::{Scalar, Vector};
use crate::query::{QueryExecutionStats, QueryRegion, execute_query_with_stats};
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
/// through the baseline abstraction.
pub fn compare_query_execution(
    index: &FSEIndex,
    points: &[Vector],
    query: &QueryRegion,
) -> QueryComparisonReport {
    let baseline = FlatScanBaseline;

    compare_query_execution_with_baseline(
        index,
        points,
        query,
        &baseline,
        &RepeatedTimingConfig::default(),
    )
}

/// Compares FSE query execution against the default flat scan baseline with repeated timing.
pub fn compare_query_execution_repeated(
    index: &FSEIndex,
    points: &[Vector],
    query: &QueryRegion,
    timing_config: &RepeatedTimingConfig,
) -> QueryComparisonReport {
    let baseline = FlatScanBaseline;

    compare_query_execution_with_baseline(index, points, query, &baseline, timing_config)
}

/// Compares FSE query execution against a supplied baseline.
///
/// # Runtime Role
///
/// This function is the extension point for future exact range-query baselines
/// such as KD-tree and R-tree implementations.
///
/// # Panics
///
/// Panics when the FSE result set differs from the baseline result set.
pub fn compare_query_execution_with_baseline(
    index: &FSEIndex,
    points: &[Vector],
    query: &QueryRegion,
    baseline: &impl RangeQueryBaseline,
    timing_config: &RepeatedTimingConfig,
) -> QueryComparisonReport {
    let (baseline_report, baseline_elapsed) = measure_elapsed(|| baseline.execute(points, query));

    let (fse_report, fse_elapsed) = measure_elapsed(|| execute_query_with_stats(index, query));

    let mut baseline_results = baseline_report.results;
    let mut fse_results = fse_report.results;

    sort_points(&mut baseline_results);
    sort_points(&mut fse_results);

    assert_eq!(
        fse_results, baseline_results,
        "FSE query results must match baseline query results"
    );

    let repeated_timing = RepeatedComparisonTimingReport {
        flat_scan: measure_repeated(timing_config, || {
            let _ = baseline.execute(points, query);
        }),
        fse: measure_repeated(timing_config, || {
            let _ = execute_query_with_stats(index, query);
        }),
    };

    let single_run_timing_ratio = duration_ratio(baseline_elapsed, fse_elapsed);
    let average_timing_ratio = duration_ratio(
        repeated_timing.flat_scan.average_elapsed,
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
            flat_scan_elapsed: baseline_elapsed,
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

fn sort_points(points: &mut [Vector]) {
    points.sort_by(|left, right| {
        for (left_value, right_value) in left.values.iter().zip(&right.values) {
            match left_value.partial_cmp(right_value) {
                Some(std::cmp::Ordering::Equal) => continue,
                Some(ordering) => return ordering,
                None => return std::cmp::Ordering::Equal,
            }
        }

        left.values.len().cmp(&right.values.len())
    });
}
