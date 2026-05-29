//! Query-to-bounds classification.

use crate::math::BoundingBox;

use super::QueryRegion;
use crate::query::region::classification::QueryBoundsClassification;

impl QueryRegion {
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
    #[inline]
    pub fn classify_bounds(&self, bounds: &BoundingBox) -> QueryBoundsClassification {
        let dimensions = self.dimensions();

        if dimensions != bounds.dimensions() {
            return QueryBoundsClassification::Disjoint;
        }

        // current benchmark path is 2d so keep it out of the generic loop
        match dimensions {
            1 => self.classify_1d_bounds(bounds),
            2 => self.classify_2d_bounds(bounds),
            _ => self.classify_nd_bounds(bounds, dimensions),
        }
    }

    #[inline]
    fn classify_1d_bounds(&self, bounds: &BoundingBox) -> QueryBoundsClassification {
        let query_min_0 = self.min[0];
        let query_max_0 = self.max[0];
        let bounds_min_0 = bounds.min[0];
        let bounds_max_0 = bounds.max[0];

        if query_max_0 < bounds_min_0 || query_min_0 > bounds_max_0 {
            return QueryBoundsClassification::Disjoint;
        }

        if query_min_0 <= bounds_min_0 && query_max_0 >= bounds_max_0 {
            QueryBoundsClassification::Covered
        } else {
            QueryBoundsClassification::Partial
        }
    }

    #[inline]
    fn classify_2d_bounds(&self, bounds: &BoundingBox) -> QueryBoundsClassification {
        let query_min_0 = self.min[0];
        let query_max_0 = self.max[0];
        let query_min_1 = self.min[1];
        let query_max_1 = self.max[1];

        let bounds_min_0 = bounds.min[0];
        let bounds_max_0 = bounds.max[0];
        let bounds_min_1 = bounds.min[1];
        let bounds_max_1 = bounds.max[1];

        if query_max_0 < bounds_min_0
            || query_min_0 > bounds_max_0
            || query_max_1 < bounds_min_1
            || query_min_1 > bounds_max_1
        {
            return QueryBoundsClassification::Disjoint;
        }

        if query_min_0 <= bounds_min_0
            && query_max_0 >= bounds_max_0
            && query_min_1 <= bounds_min_1
            && query_max_1 >= bounds_max_1
        {
            QueryBoundsClassification::Covered
        } else {
            QueryBoundsClassification::Partial
        }
    }

    #[inline]
    fn classify_nd_bounds(
        &self,
        bounds: &BoundingBox,
        dimensions: usize,
    ) -> QueryBoundsClassification {
        let mut fully_contains_bounds = true;

        for dimension in 0..dimensions {
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
}
