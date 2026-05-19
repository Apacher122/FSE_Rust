//! Query region representation.

use crate::math::{BoundingBox, Scalar, Vector};

/// Geometric relationship between a query region and a bounding box.
///
/// # Runtime Role
///
/// Traversal needs to know whether a node can be pruned, retained as fully
/// covered, or descended into as a partial overlap. Keeping this as one enum
/// lets traversal get that answer from one bounds pass instead of calling
/// separate containment and intersection checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryBoundsClassification {
    /// The query and bounds do not overlap.
    Disjoint,

    /// The query intersects the bounds but does not fully contain them.
    Partial,

    /// The query fully contains the bounds.
    Covered,
}

impl QueryBoundsClassification {
    /// Returns true when the bounds can be safely pruned.
    pub fn is_disjoint(self) -> bool {
        matches!(self, Self::Disjoint)
    }

    /// Returns true when the bounds overlap but still need exact handling below.
    pub fn is_partial(self) -> bool {
        matches!(self, Self::Partial)
    }

    /// Returns true when the query fully contains the bounds.
    pub fn is_covered(self) -> bool {
        matches!(self, Self::Covered)
    }
}

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
    /// [`QueryRegion::classify_bounds`] when it needs traversal classification
    /// and [`QueryRegion::intersects_bounds`] when it only needs intersection.
    pub fn as_bounds(&self) -> BoundingBox {
        BoundingBox::new(self.min.clone(), self.max.clone())
    }

    /// Classifies the relationship between this query and a bounding box.
    ///
    /// # Runtime Role
    ///
    /// This is the traversal hot-path bounds test. It combines containment and
    /// intersection into one dimensional pass:
    ///
    /// - `Disjoint` means the subtree can be pruned.
    /// - `Covered` means the subtree can be retained without further bounds math.
    /// - `Partial` means traversal must continue or the leaf must use exact row
    ///   evaluation.
    ///
    /// Boundary contact counts as intersection.
    pub fn classify_bounds(&self, bounds: &BoundingBox) -> QueryBoundsClassification {
        if self.dimensions() != bounds.dimensions() {
            return QueryBoundsClassification::Disjoint;
        }

        let mut fully_contains_bounds = true;

        // one pass answers both questions now
        for dimension in 0..self.dimensions() {
            let query_min = self.min[dimension];
            let query_max = self.max[dimension];
            let bounds_min = bounds.min[dimension];
            let bounds_max = bounds.max[dimension];

            if query_max < bounds_min || query_min > bounds_max {
                return QueryBoundsClassification::Disjoint;
            }

            if query_min > bounds_min || query_max < bounds_max {
                fully_contains_bounds = false;
            }
        }

        if fully_contains_bounds {
            QueryBoundsClassification::Covered
        } else {
            QueryBoundsClassification::Partial
        }
    }

    /// Returns true when this query fully contains a bounding box.
    ///
    /// # Runtime Role
    ///
    /// This method supports retained-leaf classification during traversal. If a
    /// query fully contains a leaf bounding box, every reconstructed row from
    /// that leaf is guaranteed to satisfy the query.
    pub fn contains_bounds(&self, bounds: &BoundingBox) -> bool {
        self.classify_bounds(bounds).is_covered()
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
        !self.classify_bounds(bounds).is_disjoint()
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
    /// Retained-leaf execution already knows that reconstructed scratch buffers
    /// match the query dimensionality. This helper avoids repeating the public
    /// length check for every reconstructed candidate row.
    ///
    /// Boundary values are treated as contained.
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

        for dimension in 0..dimensions {
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
