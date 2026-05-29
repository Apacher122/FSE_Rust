//! Query region construction and shape helpers.

use crate::math::{BoundingBox, Scalar};

use super::QueryRegion;

impl QueryRegion {
    /// Creates a query region from explicit minimum and maximum coordinates.
    ///
    /// # Panics
    ///
    /// Panics when minimum and maximum vectors have different dimensionality or
    /// when no dimensions are provided.
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
    ///
    /// # Notes
    ///
    /// This allocates a new bounding box. Hot traversal code should prefer
    /// [`QueryRegion::classify_bounds`] when it needs traversal classification
    /// and [`QueryRegion::intersects_bounds`] when it only needs intersection.
    pub fn as_bounds(&self) -> BoundingBox {
        BoundingBox::new(self.min.clone(), self.max.clone())
    }
}
