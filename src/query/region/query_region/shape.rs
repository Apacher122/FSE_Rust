//! Query region data shape.

use crate::math::Scalar;

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
/// This structure corresponds to an axis-aligned query region $Q$.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryRegion {
    /// Minimum coordinate value per query dimension.
    pub min: Vec<Scalar>,

    /// Maximum coordinate value per query dimension.
    pub max: Vec<Scalar>,
}
