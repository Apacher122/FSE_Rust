//! Bounding box data shape.

use crate::math::Scalar;

/// Axis-aligned bounding box used for partition-level pruning.
///
/// # Runtime Role
///
/// `BoundingBox` stores the minimum and maximum coordinate value for each
/// dimension of a partition.
///
/// # Formal Reference
///
/// This structure corresponds to the bounded support region $B_k$.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundingBox {
    /// Minimum coordinate value per dimension.
    pub min: Vec<Scalar>,

    /// Maximum coordinate value per dimension.
    pub max: Vec<Scalar>,
}
