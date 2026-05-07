//! Flat scan baseline query execution.

use crate::math::Vector;
use crate::query::QueryRegion;

/// Runtime statistics collected during flat scan execution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FlatScanStats {
    /// Number of records evaluated by the scan.
    pub evaluated_records: usize,
    /// Number of records returned by the query.
    pub matched_records: usize,
}

/// Flat scan result paired with execution statistics.
pub struct FlatScanReport {
    /// Exact query matches.
    pub results: Vec<Vector>,
    /// Runtime statistics for the scan.
    pub stats: FlatScanStats,
}

/// Executes a plain linear scan over all points.
///
/// # Runtime Role
///
/// This function provides the baseline execution path equivalent to evaluating
/// every record directly against the query predicate.
///
/// # Formal Reference
///
/// This corresponds to the scan complexity baseline `T_scan(N) = O(N)`.
pub fn flat_scan(points: &[Vector], query: &QueryRegion) -> Vec<Vector> {
    let mut results = Vec::new();
    for point in points {
        if query.contains_point(point) {
            results.push(point.clone());
        }
    }
    results
}

/// Executes a plain linear scan and returns execution statistics.
///
/// # Runtime Role
///
/// This function is useful for comparing conventional scan work against FSE
/// query execution work.
pub fn flat_scan_with_stats(points: &[Vector], query: &QueryRegion) -> FlatScanReport {
    let mut results = Vec::new();
    let mut stats = FlatScanStats::default();

    for point in points {
        stats.evaluated_records += 1;
        if query.contains_point(point) {
            stats.matched_records += 1;
            results.push(point.clone());
        }
    }
    FlatScanReport { results, stats }
}
