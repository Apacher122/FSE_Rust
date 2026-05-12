//! Query region representation.

use crate::math::{BoundingBox, Scalar, Vector};

/// Axis-aligned query region.
///
/// # Runtime Role
///
/// `QueryRegion` represents an admissible query region that can be evaluated
/// against partition bounding boxes during metadata traversal.
///
/// # Formal Reference
///
/// This structure corresponds to an axis-aligned query region `Q`.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryRegion {
    /// Minimum coordinate value per query dimension.
    pub min: Vec<Scalar>,

    /// Maximum coordinate value per query dimension.
    pub max: Vec<Scalar>,
}

impl QueryRegion {
    /// Creates a query region from explicit minimum and maximum coordinates.
    ///
    /// # Panics
    ///
    /// Panics when minimum and maximum vectors have different dimensionality.
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

    /// Returns the number of dimensions represented by the query region.
    pub fn dimensions(&self) -> usize {
        self.min.len()
    }

    /// Converts the query region into a bounding box.
    ///
    /// # Runtime Role
    ///
    /// This allows query-box intersection to reuse the same bounded-region logic
    /// used by partition metadata.
    pub fn as_bounds(&self) -> BoundingBox {
        BoundingBox::new(self.min.clone(), self.max.clone())
    }

    /// Returns true when the point lies inside the query region.
    ///
    /// Boundary values are treated as contained.
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
