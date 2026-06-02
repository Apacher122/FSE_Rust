//! Row reconstruction helpers.

use crate::math::Scalar;
use crate::storage::PartitionNode;

use super::shape::validate_partition_reconstruction_shape;

/// Reconstructs one row from a partition into an existing coordinate buffer.
///
/// # Runtime Role
///
/// This function performs row-local deferred reconstruction without allocating
/// a new vector for every candidate record. The caller owns the output buffer
/// and may reuse it across many rows.
///
/// # Formal Reference
///
/// This implements the reconstruction operator $\Phi_k(\Delta) = \mu_k + \Delta$
/// for one residual row.
///
/// # Panics
///
/// Panics when centroid and residual dimensionality are inconsistent, when the
/// residual row is out of range, when residual dimensions have inconsistent row
/// counts, or when reconstruction produces a non-finite coordinate.
pub fn reconstruct_row_into(node: &PartitionNode, row: usize, output: &mut Vec<Scalar>) {
    let shape = validate_partition_reconstruction_shape(node);

    assert!(
        row < shape.cardinality,
        "residual row index must be inside the partition cardinality"
    );

    reconstruct_row_into_prevalidated(node, row, shape.dimensions, output);
    assert_reconstructed_values_are_finite(output);
}

/// Reconstructs one row after the caller has already validated partition shape.
///
/// # Runtime Role
///
/// This is the retained-leaf hot-path variant. It avoids repeating the public
/// shape checks for every row in a leaf while preserving debug assertions that
/// catch misuse during development.
///
/// # Panics
///
/// In release builds, this function relies on the caller to pass a valid row
/// and a previously validated partition shape. Invalid inputs may still panic
/// through normal slice indexing.
#[inline]
pub(crate) fn reconstruct_row_into_prevalidated(
    node: &PartitionNode,
    row: usize,
    dimensions: usize,
    output: &mut Vec<Scalar>,
) {
    debug_assert_eq!(
        node.centroid.len(),
        dimensions,
        "prevalidated centroid dimensionality should match"
    );
    debug_assert_eq!(
        node.residuals.dimensions.len(),
        dimensions,
        "prevalidated residual dimensionality should match"
    );
    debug_assert!(
        row < node.residuals.cardinality(),
        "prevalidated residual row should be inside cardinality"
    );

    output.clear();

    if output.capacity() < dimensions {
        output.reserve(dimensions - output.capacity());
    }

    // keep this as the restored buffered row path
    // commit 120 showed the 2d row branch wasnt worth keeping
    for (centroid_value, residual_dimension) in node.centroid.iter().zip(&node.residuals.dimensions)
    {
        output.push(*centroid_value + residual_dimension[row]);
    }
}

fn assert_reconstructed_values_are_finite(values: &[Scalar]) {
    for value in values {
        assert!(
            value.is_finite(),
            "reconstructed coordinate values must be finite"
        );
    }
}
