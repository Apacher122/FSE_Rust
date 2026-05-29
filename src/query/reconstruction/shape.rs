//! Reconstruction shape validation.

use crate::storage::PartitionNode;

/// Validated reconstruction shape for a partition.
///
/// # Runtime Role
///
/// This small value lets hot execution paths validate partition reconstruction
/// shape once and then reuse the dimensionality and row count across every row
/// in the retained leaf.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReconstructionShape {
    /// Number of coordinate dimensions reconstructed for each row.
    pub dimensions: usize,

    /// Number of residual rows available for reconstruction.
    pub cardinality: usize,
}

/// Validates the reconstruction shape of a partition.
///
/// # Runtime Role
///
/// Public reconstruction helpers must defend against malformed partition state.
/// Retained-leaf execution can call this once per leaf and then use the
/// prevalidated row reconstruction helpers inside the row loop.
///
/// # Panics
///
/// Panics when centroid and residual dimensionality are inconsistent or when
/// residual dimensions do not contain the same row count.
pub(crate) fn validate_partition_reconstruction_shape(node: &PartitionNode) -> ReconstructionShape {
    let dimensions = node.residuals.dimensions();
    let cardinality = node.residuals.cardinality();

    assert_eq!(
        node.centroid.len(),
        dimensions,
        "partition centroid and residual dimensionality must match"
    );

    for (dimension_index, residual_dimension) in node.residuals.dimensions.iter().enumerate() {
        assert_eq!(
            residual_dimension.len(),
            cardinality,
            "residual dimension {dimension_index} has {} rows but expected {cardinality}",
            residual_dimension.len()
        );
    }

    ReconstructionShape {
        dimensions,
        cardinality,
    }
}
