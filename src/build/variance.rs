//! Variance utilities for partition construction.

use crate::math::{Scalar, Vector};

/// Computes the empirical variance for each dimension.
///
/// # Runtime Role
///
/// Variance is used by the initial builder to select a split axis.
///
/// # Panics
///
/// Panics when the point set is empty or dimensionality is inconsistent.
pub fn variance_by_dimension(points: &[Vector]) -> Vec<Scalar> {
    assert!(
        !points.is_empty(),
        "cannot compute variance for an empty point set"
    );

    let dimensions = points[0].dimensions();
    assert!(dimensions > 0, "points must have at least one dimension");

    let mut means = vec![0.0; dimensions];

    for point in points {
        assert_eq!(
            point.dimensions(),
            dimensions,
            "all points must have the same dimensionality"
        );

        for dimension in 0..dimensions {
            means[dimension] += point.values[dimension];
        }
    }

    let count = points.len() as Scalar;

    for mean in &mut means {
        *mean /= count;
    }

    let mut variances = vec![0.0; dimensions];

    for point in points {
        for dimension in 0..dimensions {
            let delta = point.values[dimension] - means[dimension];
            variances[dimension] += delta * delta;
        }
    }

    for variance in &mut variances {
        *variance /= count;
    }

    variances
}

/// Returns the dimension with the largest variance.
///
/// Ties resolve to the lowest dimension index for deterministic behavior.
pub fn max_variance_dimension(points: &[Vector]) -> usize {
    let variances = variance_by_dimension(points);

    let mut best_dimension = 0;
    let mut best_variance = variances[0];

    for (dimension, variance) in variances.iter().enumerate().skip(1) {
        if *variance > best_variance {
            best_dimension = dimension;
            best_variance = *variance;
        }
    }

    best_dimension
}
