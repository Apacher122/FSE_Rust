//! Leaf record bounds validation diagnostics.

use crate::build::validation::{bounds_ranges_are_valid, value_is_inside_leaf_bounds};
use crate::storage::{FSEIndex, PartitionNode};

use super::types::LeafRecordBoundsViolation;

pub(super) fn leaf_record_bounds_violations(index: &FSEIndex) -> Vec<LeafRecordBoundsViolation> {
    let mut violations = Vec::new();

    if index.nodes.is_empty() || index.root >= index.nodes.len() {
        return violations;
    }

    for node in index.nodes.iter().filter(|node| node.is_leaf) {
        collect_leaf_record_bounds_violations(node, &mut violations);
    }

    violations
}

fn collect_leaf_record_bounds_violations(
    node: &PartitionNode,
    violations: &mut Vec<LeafRecordBoundsViolation>,
) {
    let dimensions = node.centroid.len();

    if dimensions == 0 {
        return;
    }

    if node.bounds.min.len() != dimensions
        || node.bounds.max.len() != dimensions
        || node.residuals.dimensions() != dimensions
    {
        return;
    }

    if !node.residuals.has_consistent_shape() {
        return;
    }

    if !bounds_ranges_are_valid(&node.bounds) {
        return;
    }

    let row_count = node.residuals.cardinality().min(node.cardinality);

    for row in 0..row_count {
        for dimension in 0..dimensions {
            let value = node.centroid[dimension] + node.residuals.dimensions[dimension][row];
            let minimum = node.bounds.min[dimension];
            let maximum = node.bounds.max[dimension];

            if !value_is_inside_leaf_bounds(value, minimum, maximum) {
                violations.push(LeafRecordBoundsViolation {
                    node_id: node.id,
                    row,
                    dimension,
                    value,
                    minimum,
                    maximum,
                });
            }
        }
    }
}
