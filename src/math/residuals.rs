//! Centroid-relative residual storage.

use crate::math::{Scalar, Vector};

/// A structure-of-arrays (SoA) layout for storing centroid-relative residual vectors.
///
/// `ResidualBlock` organizes residual coordinates by dimension rather than by individual
/// record (row). This memory layout is intentionally designed to maximize cache efficiency
/// during traversal and to seamlessly support vectorized SIMD reconstruction pipelines.
/// In the formal FSE specification, this structure represents the residual encoding
/// $\Delta_k(x) = x - \mu_k$.
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

    /// Computes a residual block from a set of absolute coordinates and a local centroid.
    ///
    /// This method converts standard positional coordinates into centroid-relative
    /// residual values by subtracting the local centroid from each point. Formally,
    /// this executes the transformation $\Delta_k(x) = x - \mu_k$.
    pub fn from_points(points: &[Vector], centroid: &[Scalar]) -> Self {
        let dimensions = centroid.len();
        let mut residuals = vec![Vec::with_capacity(points.len()); dimensions];

        for point in points {
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
