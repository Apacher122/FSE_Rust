//! Exact point-level query evaluation.

use crate::math::Vector;
use crate::query::QueryRegion;

/// Evaluates reconstructed points against a query region.
///
/// # Runtime Role
///
/// Predicate evaluation performs Stage III of the FSE query pipeline. It filters
/// reconstructed candidate records after metadata pruning and deferred
/// reconstruction have already occurred.
pub fn evaluate_query(points: &[Vector], query: &QueryRegion) -> Vec<Vector> {
    let mut matches = Vec::new();

    for point in points {
        if query.contains_point(point) {
            matches.push(point.clone());
        }
    }

    matches
}
