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
    ///
    /// # Runtime Role
    ///
    /// This constructor validates that every residual dimension contains the
    /// same number of rows. That invariant is required by reconstruction because
    /// each logical record is reconstructed by reading one value from every
    /// dimension at the same row index.
    ///
    /// # Panics
    ///
    /// Panics when residual dimensions do not contain the same number of rows.
    pub fn new(dimensions: Vec<Vec<Scalar>>) -> Self {
        Self::assert_consistent_shape(&dimensions);

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

        Self::new(residuals)
    }

    /// Returns the total number of dimensions tracked by this residual block.
    pub fn dimensions(&self) -> usize {
        self.dimensions.len()
    }

    /// Returns the number of individual records represented within the block.
    pub fn cardinality(&self) -> usize {
        self.dimensions.first().map_or(0, Vec::len)
    }

    /// Returns true when all residual dimensions contain the same number of rows.
    ///
    /// # Runtime Role
    ///
    /// This is useful for validation paths that need to inspect residual storage
    /// without constructing a new block.
    pub fn has_consistent_shape(&self) -> bool {
        let Some(first_dimension) = self.dimensions.first() else {
            return true;
        };

        let expected_rows = first_dimension.len();

        self.dimensions
            .iter()
            .all(|dimension| dimension.len() == expected_rows)
    }

    /// Returns the row count stored by each residual dimension.
    ///
    /// # Runtime Role
    ///
    /// This supports diagnostics and tests for malformed residual storage.
    pub fn dimension_lengths(&self) -> Vec<usize> {
        self.dimensions.iter().map(Vec::len).collect()
    }

    /// Checks if the residual block is completely empty.
    pub fn is_empty(&self) -> bool {
        self.cardinality() == 0
    }

    fn assert_consistent_shape(dimensions: &[Vec<Scalar>]) {
        let Some(first_dimension) = dimensions.first() else {
            return;
        };

        let expected_rows = first_dimension.len();

        // dont let one dimension quietly drift away from the others
        for (dimension_index, dimension) in dimensions.iter().enumerate().skip(1) {
            assert_eq!(
                dimension.len(),
                expected_rows,
                "residual dimension {dimension_index} has {} rows but expected {expected_rows}",
                dimension.len()
            );
        }
    }
}
