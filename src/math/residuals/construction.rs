//! Residual block construction helpers.

use crate::math::{Scalar, Vector};

use super::ResidualBlock;

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
    /// This implements $\Delta_k(x) = x - \mu_k$.
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
