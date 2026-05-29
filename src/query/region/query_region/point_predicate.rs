//! Query point predicate helpers.

use crate::math::{Scalar, Vector};

use super::QueryRegion;

impl QueryRegion {
    /// Returns true when a coordinate slice lies inside the query region.
    ///
    /// Boundary values are treated as contained.
    ///
    /// # Runtime Role
    ///
    /// This method supports allocation-conscious query execution because callers
    /// can evaluate coordinates held in a reusable reconstruction buffer.
    pub fn contains_values(&self, values: &[Scalar]) -> bool {
        let dimensions = self.dimensions();

        if values.len() != dimensions {
            return false;
        }

        self.contains_values_prevalidated(values, dimensions)
    }

    /// Returns true when a prevalidated coordinate slice lies inside the query region.
    ///
    /// # Runtime Role
    ///
    /// Retained-leaf execution already knows the reconstructed scratch buffer
    /// matches the query dimensionality. This helper avoids repeating the public
    /// length check for every candidate row.
    ///
    /// The 1D and 2D branches are intentionally explicit because the current
    /// benchmark data is 2D and this method sits in the exact predicate hot path.
    /// Higher-dimensional queries keep the general loop.
    pub(crate) fn contains_values_prevalidated(
        &self,
        values: &[Scalar],
        dimensions: usize,
    ) -> bool {
        debug_assert_eq!(
            self.dimensions(),
            dimensions,
            "prevalidated query dimensionality should match"
        );
        debug_assert_eq!(
            values.len(),
            dimensions,
            "prevalidated coordinate dimensionality should match"
        );

        match dimensions {
            1 => values[0] >= self.min[0] && values[0] <= self.max[0],
            2 => {
                values[0] >= self.min[0]
                    && values[0] <= self.max[0]
                    && values[1] >= self.min[1]
                    && values[1] <= self.max[1]
            }
            _ => {
                // keep the generic path boring it has to stay obviously correct
                for dimension in 0..dimensions {
                    let value = values[dimension];

                    if value < self.min[dimension] || value > self.max[dimension] {
                        return false;
                    }
                }

                true
            }
        }
    }

    /// Returns true when the point lies inside the query region.
    ///
    /// Boundary values are treated as contained.
    pub fn contains_point(&self, point: &Vector) -> bool {
        self.contains_values(&point.values)
    }
}
