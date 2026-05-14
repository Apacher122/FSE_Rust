use crate::benchmark::{RangeQueryBaseline, flat_scan};
use crate::math::Vector;
use crate::query::QueryRegion;

/// Sorts points lexicographically for order-independent result comparison.
pub fn sort_points(points: &mut [Vector]) {
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

/// Verifies that a baseline returns the same exact result set as flat scan.
pub fn assert_baseline_matches_flat_scan<B>(baseline: B, points: &[Vector], query: &QueryRegion)
where
    B: RangeQueryBaseline,
{
    let baseline_report = baseline.execute(query);
    let scan_results = flat_scan(points, query);

    let mut baseline_results = baseline_report.results;
    let mut expected_results = scan_results;

    sort_points(&mut baseline_results);
    sort_points(&mut expected_results);

    assert_eq!(baseline_results, expected_results);
    assert_eq!(
        baseline_report.stats.matched_records,
        expected_results.len()
    );
    assert!(baseline_report.stats.evaluated_records <= points.len());
}
