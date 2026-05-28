//! Residual block data shape.

use crate::math::Scalar;

/// Residual vectors stored in structure-of-arrays layout.
///
/// # Runtime Role
///
/// `ResidualBlock` stores residual coordinates by dimension instead of by row.
/// This layout supports cache-friendly traversal and later SIMD reconstruction.
///
/// # Formal Reference
///
/// This structure corresponds to the residual encoding
/// $\Delta_k(x) = x - \mu_k$.
#[derive(Clone, Debug, PartialEq)]
pub struct ResidualBlock {
    /// Residual values grouped by dimension.
    pub dimensions: Vec<Vec<Scalar>>,
}
