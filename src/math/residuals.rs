//! Centroid-relative residual storage.

use crate::math::{Scalar, Vector};

/// Residual vectors stored in structure-of-arrays layout.
///
/// # Runtime Role
///
/// `ResidualBlock` stores residual coordinates by dimension instead of by row.
/// This layout supports cache-friendly traversal and later SIMD reconstruction.
///
/// # Formal Reference
///
/// This structure corresponds to the residual encoding `Delta_k(x) = x - mu_k`.
#[derive(Clone, Debug, PartialEq)]
pub struct ResidualBlock {
    /// Residual values grouped by dimension.
    pub dimensions: Vec<Vec<Scalar>>,
}

impl ResidualBlock {
    /// Creates a new residual block from residual values.
    pub fn new(dimensions: Vec<Vec<Scalar>>) -> Self {
        Self { dimensions }
    }

    /// Builds a residual block from points and a centroid.
    ///
    /// # Runtime Role
    ///
    /// Converts absolute coordinates into centroid-relative residual values.
    ///
    /// # Formal Reference
    ///
    /// This implements `Delta_k(x) = x - mu_k`.
    ///
    /// # Panics
    ///
    /// Panics when dimensionality is inconsistent.
    pub fn from_points(points: &[Vector], centroid: &[Scalar]) -> Self {
        let dimensions = centroid.len();
        assert!(dimensions > 0, "centroid must have at least one dimension");
        let mut residuals = vec![Vec::with_capacity(points.len()); dimensions];

        for point in points {
            assert_eq!(
                point.dimensions(),
                dimensions,
                "point and centroid dimensionality must match"
            );
            for dimension in 0..dimensions {
                residuals[dimension].push(point.values[dimension] - centroid[dimension]);
            }
        }
        Self {
            dimensions: residuals,
        }
    }

    /// Returns the total number of dimensions tracked by this residual block.
    pub fn dimensions(&self) -> usize {
        self.dimensions.len()
    }

    /// Returns the number of individual records (rows) represented within the block.
    pub fn cardinality(&self) -> usize {
        self.dimensions.first().map_or(0, Vec::len)
    }

    /// Checks if the residual block is completely empty (contains zero records).
    pub fn is_empty(&self) -> bool {
        self.cardinality() == 0
    }
}
