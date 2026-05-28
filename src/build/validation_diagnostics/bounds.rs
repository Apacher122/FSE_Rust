//! Parent-child bounds validation diagnostics.

use crate::storage::FSEIndex;

use super::types::ParentChildBoundsViolation;

pub(super) fn parent_child_bounds_violations(index: &FSEIndex) -> Vec<ParentChildBoundsViolation> {
    let mut violations = Vec::new();

    if index.nodes.is_empty() || index.root >= index.nodes.len() {
        return violations;
    }

    for node in &index.nodes {
        for child_id in &node.children {
            if *child_id >= index.nodes.len() {
                continue;
            }

            let child = &index.nodes[*child_id];

            if !node.bounds.contains_bounds(&child.bounds) {
                violations.push(ParentChildBoundsViolation {
                    parent_id: node.id,
                    child_id: *child_id,
                });
            }
        }
    }

    violations
}
