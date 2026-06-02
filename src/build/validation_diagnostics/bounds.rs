//! Parent-child bounds validation diagnostics.

use crate::build::validation::bounds_ranges_are_valid;
use crate::storage::FSEIndex;

use super::types::ParentChildBoundsViolation;

pub(super) fn parent_child_bounds_violations(index: &FSEIndex) -> Vec<ParentChildBoundsViolation> {
    let mut violations = Vec::new();

    if index.nodes.is_empty() || index.root >= index.nodes.len() {
        return violations;
    }

    for node in &index.nodes {
        if !bounds_ranges_are_valid(&node.bounds) {
            violations.extend(node.children.iter().filter_map(|child_id| {
                (*child_id < index.nodes.len()).then_some(ParentChildBoundsViolation {
                    parent_id: node.id,
                    child_id: *child_id,
                })
            }));

            continue;
        }

        for child_id in &node.children {
            if *child_id >= index.nodes.len() {
                continue;
            }

            let child = &index.nodes[*child_id];

            if !bounds_ranges_are_valid(&child.bounds) {
                violations.push(ParentChildBoundsViolation {
                    parent_id: node.id,
                    child_id: *child_id,
                });

                continue;
            }

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
