//! Leaf ownership cardinality validation diagnostics.

use crate::storage::FSEIndex;

use super::types::{
    LeafOwnershipCardinalityDiagnostics, LeafOwnershipCardinalityViolation,
    LeafOwnershipParentCountViolation,
};

pub(super) fn leaf_ownership_cardinality_diagnostics(
    index: &FSEIndex,
) -> LeafOwnershipCardinalityDiagnostics {
    let parent_counts = parent_counts(index);

    LeafOwnershipCardinalityDiagnostics {
        parent_count_violations: parent_count_violations(index, &parent_counts),
        cardinality_violations: cardinality_violations(index),
        unowned_node_ids: unowned_node_ids(index),
    }
}

fn parent_counts(index: &FSEIndex) -> Vec<usize> {
    let mut parent_counts = vec![0usize; index.nodes.len()];

    for node in &index.nodes {
        for child_id in &node.children {
            if *child_id < index.nodes.len() {
                parent_counts[*child_id] += 1;
            }
        }
    }

    parent_counts
}

fn parent_count_violations(
    index: &FSEIndex,
    parent_counts: &[usize],
) -> Vec<LeafOwnershipParentCountViolation> {
    parent_counts
        .iter()
        .enumerate()
        .filter_map(|(node_id, parent_count)| {
            let expected_parent_count = if node_id == index.root && index.root < index.nodes.len() {
                0
            } else {
                1
            };

            (*parent_count != expected_parent_count).then_some(LeafOwnershipParentCountViolation {
                node_id,
                parent_count: *parent_count,
                expected_parent_count,
            })
        })
        .collect()
}

fn cardinality_violations(index: &FSEIndex) -> Vec<LeafOwnershipCardinalityViolation> {
    let mut violations = Vec::new();

    for node_id in 0..index.nodes.len() {
        let node = &index.nodes[node_id];

        if node.is_leaf {
            continue;
        }

        let mut visiting = vec![false; index.nodes.len()];

        let Some(owned_leaf_cardinality) = owned_leaf_cardinality(index, node_id, &mut visiting)
        else {
            continue;
        };

        if owned_leaf_cardinality != node.cardinality {
            violations.push(LeafOwnershipCardinalityViolation {
                node_id,
                cardinality: node.cardinality,
                owned_leaf_cardinality,
            });
        }
    }

    violations
}

fn owned_leaf_cardinality(
    index: &FSEIndex,
    node_id: usize,
    visiting: &mut [bool],
) -> Option<usize> {
    if node_id >= index.nodes.len() || visiting[node_id] {
        return None;
    }

    let node = &index.nodes[node_id];

    if node.is_leaf {
        return Some(node.cardinality);
    }

    visiting[node_id] = true;

    let mut cardinality = 0usize;

    for child_id in &node.children {
        cardinality =
            cardinality.checked_add(owned_leaf_cardinality(index, *child_id, visiting)?)?;
    }

    visiting[node_id] = false;

    Some(cardinality)
}

fn unowned_node_ids(index: &FSEIndex) -> Vec<usize> {
    if index.nodes.is_empty() || index.root >= index.nodes.len() {
        return (0..index.nodes.len()).collect();
    }

    let mut visited = vec![false; index.nodes.len()];
    let mut stack = vec![index.root];

    while let Some(node_id) = stack.pop() {
        if node_id >= index.nodes.len() || visited[node_id] {
            continue;
        }

        visited[node_id] = true;

        for child_id in &index.nodes[node_id].children {
            if *child_id < index.nodes.len() {
                stack.push(*child_id);
            }
        }
    }

    visited
        .iter()
        .enumerate()
        .filter_map(|(node_id, visited)| (!visited).then_some(node_id))
        .collect()
}
