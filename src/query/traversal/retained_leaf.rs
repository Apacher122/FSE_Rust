//! Retained leaf types produced by query traversal.

use crate::storage::{FSEIndex, LeafReconstructionShape};

/// Coverage classification for a retained leaf.
///
/// # Runtime Role
///
/// `RetainedLeafCoverage` describes the relationship between the query region
/// and the retained leaf bounds.
///
/// Covered leaves do not need per-row predicate checks. Partial leaves still
/// need exact predicate checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetainedLeafCoverage {
    /// The query fully contains the retained leaf bounds.
    Covered,

    /// The query intersects the retained leaf bounds but does not fully contain them.
    Partial,
}

/// A retained leaf and its traversal-time coverage classification.
///
/// # Runtime Role
///
/// `RetainedLeaf` is the Stage I output consumed by retained-leaf execution. It
/// stores a leaf node id and the coverage classification produced by traversal.
///
/// Traversal-produced values may also store cached reconstruction shape
/// metadata. Id-only constructors are kept for tests and compatibility helpers.
#[derive(Clone, Copy, Debug)]
pub struct RetainedLeaf {
    /// Retained leaf node identifier.
    pub node_id: usize,

    /// Coverage classification for this retained leaf.
    pub coverage: RetainedLeafCoverage,

    cached_shape: Option<LeafReconstructionShape>,
}

impl PartialEq for RetainedLeaf {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id && self.coverage == other.coverage
    }
}

impl Eq for RetainedLeaf {}

impl RetainedLeaf {
    /// Creates a retained leaf classified as covered.
    ///
    /// # Runtime Role
    ///
    /// This constructor does not store reconstruction shape metadata.
    pub fn covered(node_id: usize) -> Self {
        Self {
            node_id,
            coverage: RetainedLeafCoverage::Covered,
            cached_shape: None,
        }
    }

    /// Creates a retained leaf classified as partially covered.
    ///
    /// # Runtime Role
    ///
    /// This constructor does not store reconstruction shape metadata.
    pub fn partial(node_id: usize) -> Self {
        Self {
            node_id,
            coverage: RetainedLeafCoverage::Partial,
            cached_shape: None,
        }
    }

    /// Creates a covered retained leaf with cached reconstruction shape.
    pub(crate) fn covered_with_shape(shape: LeafReconstructionShape) -> Self {
        Self::with_shape(shape.node_id, RetainedLeafCoverage::Covered, shape)
    }

    /// Creates a partially covered retained leaf with cached reconstruction shape.
    ///
    /// # Runtime Role
    ///
    /// This is currently only needed by test compatibility helpers that classify
    /// retained leaf ids after traversal.
    #[cfg(test)]
    pub(crate) fn partial_with_shape(shape: LeafReconstructionShape) -> Self {
        Self::with_shape(shape.node_id, RetainedLeafCoverage::Partial, shape)
    }

    /// Creates a retained leaf with explicit coverage and cached shape.
    pub(crate) fn with_shape(
        node_id: usize,
        coverage: RetainedLeafCoverage,
        shape: LeafReconstructionShape,
    ) -> Self {
        debug_assert_eq!(
            node_id, shape.node_id,
            "retained leaf node id should match cached shape node id"
        );

        Self {
            node_id,
            coverage,
            cached_shape: Some(shape),
        }
    }

    /// Returns true when traversal proved full query coverage.
    pub fn is_covered(&self) -> bool {
        matches!(self.coverage, RetainedLeafCoverage::Covered)
    }

    /// Returns true when the retained leaf still requires exact predicate checks.
    pub fn is_partial(&self) -> bool {
        matches!(self.coverage, RetainedLeafCoverage::Partial)
    }

    /// Returns cached reconstruction shape for this retained leaf.
    ///
    /// # Runtime Role
    ///
    /// Traversal-produced retained leaves already carry shape metadata. Older
    /// tests and compatibility helpers may construct id-only retained leaves, so
    /// this method falls back to the index cache when needed.
    pub(crate) fn reconstruction_shape(&self, index: &FSEIndex) -> LeafReconstructionShape {
        self.cached_shape
            .unwrap_or_else(|| index.leaf_reconstruction_shape(self.node_id))
    }
}
