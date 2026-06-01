//! Leaf record bounds validation.

use crate::math::Scalar;
use crate::storage::{FSEIndex, PartitionNode};

const BOUNDS_TOLERANCE_FACTOR: Scalar = 8.0;

/// Validates that every reconstructed leaf row is inside its leaf bounds.
///
/// # Validation Rule
///
/// For every leaf partition `P_k`, each reconstructed row `mu_k + Delta_k`
/// must be contained by `B_k` within scalar rounding tolerance.
pub fn validate_leaf_record_bounds(index: &FSEIndex) -> bool {
    if index.nodes.is_empty() || index.root >= index.nodes.len() {
        return false;
    }

    index
        .nodes
        .iter()
        .filter(|node| node.is_leaf)
        .all(leaf_records_are_inside_bounds)
}

fn leaf_records_are_inside_bounds(node: &PartitionNode) -> bool {
    let dimensions = node.centroid.len();

    if dimensions == 0 {
        return false;
    }

    if node.bounds.dimensions() != dimensions || node.residuals.dimensions() != dimensions {
        return false;
    }

    if !node.residuals.has_consistent_shape() {
        return false;
    }

    if node.residuals.cardinality() != node.cardinality {
        return false;
    }

    if !bounds_ranges_are_valid(&node.bounds.min, &node.bounds.max) {
        return false;
    }

    for row in 0..node.cardinality {
        if !reconstructed_row_is_inside_bounds(node, row, dimensions) {
            return false;
        }
    }

    true
}

fn bounds_ranges_are_valid(min: &[Scalar], max: &[Scalar]) -> bool {
    min.iter()
        .zip(max)
        .all(|(minimum, maximum)| minimum.is_finite() && maximum.is_finite() && minimum <= maximum)
}

fn reconstructed_row_is_inside_bounds(node: &PartitionNode, row: usize, dimensions: usize) -> bool {
    for dimension in 0..dimensions {
        let value = node.centroid[dimension] + node.residuals.dimensions[dimension][row];

        if !value_is_inside_leaf_bounds(
            value,
            node.bounds.min[dimension],
            node.bounds.max[dimension],
        ) {
            return false;
        }
    }

    true
}

pub(crate) fn value_is_inside_leaf_bounds(value: Scalar, minimum: Scalar, maximum: Scalar) -> bool {
    if !value.is_finite() {
        return false;
    }

    let tolerance = bounds_tolerance(value, minimum, maximum);

    value >= minimum - tolerance && value <= maximum + tolerance
}

fn bounds_tolerance(value: Scalar, minimum: Scalar, maximum: Scalar) -> Scalar {
    let scale = value.abs().max(minimum.abs()).max(maximum.abs()).max(1.0);

    Scalar::EPSILON * scale * BOUNDS_TOLERANCE_FACTOR
}
