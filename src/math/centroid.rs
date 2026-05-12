//! Centroid calculation utilities.

use crate::math::{Scalar, Vector};

/// Computes the geometric centroid for a non-empty set of points.
///
/// # Runtime Role
///
/// The centroid acts as the local reference point for residual encoding within
/// a partition.
///
/// # Formal Reference
///
/// This implements the partition centroid `mu_k`.
///
/// # Panics
///
/// Panics when the point set is empty or dimensionality is inconsistent.
pub fn compute_centroid(points: &[Vector]) -> Vec<Scalar> {
    assert!(
        !points.is_empty(),
        "cannot compute centroid for empty points"
    );
    let dimensions = points[0].dimensions();
    assert!(dimensions > 0, "points must have at least one dimension");
    let mut centroid = vec![0.0; dimensions];

    for point in points {
        assert_eq!(
            point.dimensions(),
            dimensions,
            "all points must have the same dimensionality"
        );

        for dimension in 0..dimensions {
            centroid[dimension] += point.values[dimension];
        }
    }

    let count = points.len() as Scalar;
    for value in &mut centroid {
        *value /= count;
    }
    centroid
}
