//! Query/bounds classification types.

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
