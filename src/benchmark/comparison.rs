//! Comparison utilities for FSE and flat scan execution.

use crate::benchmark::{FlatScanStats, flat_scan_with_stats};
use crate::math::{Scalar, Vector};
use crate::query::{QueryExecutionStats, QueryRegion, execute_query_with_stats};
use crate::storage::FSEIndex;

/// Side-by-side report comparing FSE query execution with flat scan execution.
///
/// # Runtime Role
///
/// `QueryComparisonReport` is intended for early correctness and performance
/// analysis. It does not measure wall-clock time. Instead, it compares logical
/// execution work between the baseline scan path and the FSE query path.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryComparisonReport {
    /// Statistics from the flat scan baseline.
    pub scan_stats: FlatScanStats,

    /// Statistics from the FSE execution path.
    pub fse_stats: QueryExecutionStats,

    /// Number of records avoided by FSE reconstruction relative to flat scan evaluation.
    pub avoided_reconstructions: usize,

    /// Fraction of baseline record evaluations avoided by FSE reconstruction.
    pub reconstruction_avoidance_ratio: Scalar,
}

/// Compares FSE query execution against flat scan execution.
///
/// # Runtime Role
///
/// This function runs both execution paths and verifies they produce the same
/// exact result set before returning a comparison report.
///
/// # Panics
///
/// Panics when the FSE result set differs from the flat scan result set.
pub fn compare_query_execution(
    index: &FSEIndex,
    points: &[Vector],
    query: &QueryRegion,
) -> QueryComparisonReport {
    let scan_report = flat_scan_with_stats(points, query);
    let fse_report = execute_query_with_stats(index, query);

    let mut scan_results = scan_report.results;
    let mut fse_results = fse_report.results;

    sort_points(&mut scan_results);
    sort_points(&mut fse_results);

    assert_eq!(
        fse_results, scan_results,
        "FSE query results must match flat scan results"
    );

    let evaluated_records = scan_report.stats.evaluated_records;
    let reconstructed_records = fse_report.stats.reconstructed_records;

    let avoided_reconstructions = evaluated_records.saturating_sub(reconstructed_records);

    let reconstruction_avoidance_ratio = if evaluated_records == 0 {
        0.0
    } else {
        avoided_reconstructions as Scalar / evaluated_records as Scalar
    };

    QueryComparisonReport {
        scan_stats: scan_report.stats,
        fse_stats: fse_report.stats,
        avoided_reconstructions,
        reconstruction_avoidance_ratio,
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
