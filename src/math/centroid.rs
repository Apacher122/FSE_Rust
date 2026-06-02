//! Centroid calculation utilities.

use std::error::Error;
use std::fmt;

use crate::math::{Scalar, Vector};

/// Error returned when checked centroid computation fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CentroidError {
    /// No points were provided.
    EmptyPointSet,
    /// Points had no coordinate dimensions.
    EmptyPointDimensions,
    /// A point had a different dimensionality than the first point.
    DimensionMismatch {
        /// Point containing the mismatched dimensionality.
        point: usize,
        /// Dimensionality found in the point.
        actual_dimensions: usize,
        /// Dimensionality expected from the first point.
        expected_dimensions: usize,
    },
    /// A point coordinate was not finite.
    NonFiniteCoordinate {
        /// Point containing the non-finite coordinate.
        point: usize,
        /// Dimension containing the non-finite coordinate.
        dimension: usize,
    },
    /// A computed centroid value was not finite.
    NonFiniteCentroid {
        /// Dimension containing the non-finite centroid value.
        dimension: usize,
    },
}

impl fmt::Display for CentroidError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPointSet => formatter.write_str("cannot compute centroid for empty points"),
            Self::EmptyPointDimensions => {
                formatter.write_str("points must have at least one dimension")
            }
            Self::DimensionMismatch { .. } => {
                formatter.write_str("all points must have the same dimensionality")
            }
            Self::NonFiniteCoordinate { .. } => {
                formatter.write_str("point coordinates must be finite")
            }
            Self::NonFiniteCentroid { .. } => formatter.write_str("centroid values must be finite"),
        }
    }
}

impl Error for CentroidError {}

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
/// Panics when the point set is empty, dimensionality is inconsistent, a point
/// coordinate is not finite, or a computed centroid value is not finite.
pub fn compute_centroid(points: &[Vector]) -> Vec<Scalar> {
    try_compute_centroid(points).unwrap_or_else(|error| panic!("{error}"))
}

/// Computes the geometric centroid and returns an error when points are invalid.
///
/// # Runtime Role
///
/// This function is intended for caller-facing input validation. It enforces
/// the same invariants as [`compute_centroid`] without panicking.
pub fn try_compute_centroid(points: &[Vector]) -> Result<Vec<Scalar>, CentroidError> {
    if points.is_empty() {
        return Err(CentroidError::EmptyPointSet);
    }

    let dimensions = points[0].dimensions();

    if dimensions == 0 {
        return Err(CentroidError::EmptyPointDimensions);
    }

    let mut centroid = vec![0.0; dimensions];

    for (point_index, point) in points.iter().enumerate() {
        if point.dimensions() != dimensions {
            return Err(CentroidError::DimensionMismatch {
                point: point_index,
                actual_dimensions: point.dimensions(),
                expected_dimensions: dimensions,
            });
        }

        for dimension in 0..dimensions {
            let coordinate = point.values[dimension];

            if !coordinate.is_finite() {
                return Err(CentroidError::NonFiniteCoordinate {
                    point: point_index,
                    dimension,
                });
            }

            centroid[dimension] += coordinate;
        }
    }

    let count = points.len() as Scalar;
    for (dimension, value) in centroid.iter_mut().enumerate() {
        *value /= count;

        if !value.is_finite() {
            return Err(CentroidError::NonFiniteCentroid { dimension });
        }
    }

    Ok(centroid)
}
