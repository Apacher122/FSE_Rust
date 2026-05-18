//! Query region representation.

use crate::math::{BoundingBox, Scalar, Vector};

/// Axis-aligned query region.
///
/// # Runtime Role
///
/// `QueryRegion` represents an admissible query region that can be evaluated
/// against partition bounding boxes during metadata traversal and against
/// reconstructed coordinate values during exact predicate evaluation.
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
    /// [`QueryRegion::intersects_bounds`] when it only needs an intersection
    /// check.
    pub fn as_bounds(&self) -> BoundingBox {
        BoundingBox::new(self.min.clone(), self.max.clone())
    }

    /// Returns true when this query fully contains a bounding box.
    ///
    /// # Runtime Role
    ///
    /// This method supports retained-leaf classification during traversal. If a
    /// query fully contains a leaf bounding box, every reconstructed row from
    /// that leaf is guaranteed to satisfy the query.
    pub fn contains_bounds(&self, bounds: &BoundingBox) -> bool {
        if self.dimensions() != bounds.dimensions() {
            return false;
        }

        self.min
            .iter()
            .zip(&self.max)
            .zip(bounds.min.iter().zip(&bounds.max))
            .all(|((query_min, query_max), (bounds_min, bounds_max))| {
                query_min <= bounds_min && query_max >= bounds_max
            })
    }

    /// Returns true when this query intersects a bounding box.
    ///
    /// # Runtime Role
    ///
    /// This method supports allocation-free metadata traversal. It checks query
    /// and node-bound overlap directly without first materializing the query as
    /// a temporary `BoundingBox`.
    ///
    /// Boundary contact counts as intersection.
    pub fn intersects_bounds(&self, bounds: &BoundingBox) -> bool {
        if self.dimensions() != bounds.dimensions() {
            return false;
        }

        // same overlap test just without building a query box
        self.min
            .iter()
            .zip(&self.max)
            .zip(bounds.min.iter().zip(&bounds.max))
            .all(|((query_min, query_max), (bounds_min, bounds_max))| {
                query_max >= bounds_min && query_min <= bounds_max
            })
    }

    /// Returns true when a coordinate slice lies inside the query region.
    ///
    /// Boundary values are treated as contained.
    ///
    /// # Runtime Role
    ///
    /// This method supports allocation-conscious query execution because callers
    /// can evaluate coordinates held in a reusable reconstruction buffer.
    pub fn contains_values(&self, values: &[Scalar]) -> bool {
        if values.len() != self.dimensions() {
            return false;
        }

        for dimension in 0..self.dimensions() {
            let value = values[dimension];

            if value < self.min[dimension] || value > self.max[dimension] {
                return false;
            }
        }

        true
    }

    /// Returns true when the point lies inside the query region.
    ///
    /// Boundary values are treated as contained.
    pub fn contains_point(&self, point: &Vector) -> bool {
        self.contains_values(&point.values)
    }
}
