//! Query region representation.

use crate::math::{BoundingBox, Scalar, Vector};

/// An axis-aligned query region used for spatial filtering.
///
/// `QueryRegion` defines an admissible geometric constraint that is evaluated
/// against partition bounding boxes during metadata traversal. In the formal
/// FSE specification, this structure corresponds to the axis-aligned query region $Q$.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryRegion {
    /// The minimum coordinate value for each query dimension.
    pub min: Vec<Scalar>,
    /// The maximum coordinate value for each query dimension.
    pub max: Vec<Scalar>,
}

impl QueryRegion {
    /// Creates a new query region from explicit minimum and maximum coordinates.
    ///
    /// # Panics
    ///
    /// Panics if the `min` and `max` vectors have different dimensionalities,
    /// or if the vectors are empty.
    pub fn new(min: Vec<Scalar>, max: Vec<Scalar>) -> Self {
        assert_eq!(
            min.len(),
            max.len(),
            "query min and max vectors must have the same dimensionality"
        );
        assert!(
            !min.is_empty(),
            "query region must have at least one dimension"
        );
        Self { min, max }
    }

    /// Returns the number of dimensions represented by this query region.
    pub fn dimensions(&self) -> usize {
        self.min.len()
    }

    /// Converts the query region into a bounding box.
    ///
    /// This conversion allows query-to-box intersection tests to reuse the
    /// standardized bounded-region logic employed by partition metadata.
    pub fn as_bounds(&self) -> BoundingBox {
        BoundingBox::new(self.min.clone(), self.max.clone())
    }

    /// Checks if a point lies entirely inside the query region.
    ///
    /// Points located exactly on the boundary are treated as contained.
    /// Returns `false` if the point's dimensionality does not match the query region.
    pub fn contains_point(&self, point: &Vector) -> bool {
        if point.dimensions() != self.dimensions() {
            return false;
        }
        for dimension in 0..self.dimensions() {
            let value = point.values[dimension];
            if value < self.min[dimension] || value > self.max[dimension] {
                return false;
            }
        }
        true
    }
}
