//! Leaf ownership cardinality validation.

use crate::storage::FSEIndex;

#[derive(Clone, Copy, Eq, PartialEq)]
enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}

/// Validates leaf ownership cardinality across the hierarchy.
///
/// # Validation Rule
///
/// Every non-root node must have exactly one parent. Every node must be
/// reachable from the root. For each internal node, `cardinality` must equal
/// the sum of the cardinalities owned by its child subtrees.
pub fn validate_leaf_ownership_cardinality(index: &FSEIndex) -> bool {
    if index.nodes.is_empty() || index.root >= index.nodes.len() {
        return false;
    }

    if !parent_counts_are_valid(index) {
        return false;
    }

    let mut states = vec![VisitState::Unvisited; index.nodes.len()];

    let Some(root_leaf_cardinality) = subtree_leaf_cardinality(index, index.root, &mut states)
    else {
        return false;
    };

    root_leaf_cardinality == index.root_node().cardinality
        && states.iter().all(|state| *state == VisitState::Visited)
}

fn parent_counts_are_valid(index: &FSEIndex) -> bool {
    let mut parent_counts = vec![0usize; index.nodes.len()];

    for node in &index.nodes {
        for child_id in &node.children {
            if *child_id >= index.nodes.len() {
                return false;
            }

            parent_counts[*child_id] += 1;
        }
    }

    if parent_counts[index.root] != 0 {
        return false;
    }

    parent_counts
        .iter()
        .enumerate()
        .all(|(node_id, parent_count)| node_id == index.root || *parent_count == 1)
}

fn subtree_leaf_cardinality(
    index: &FSEIndex,
    node_id: usize,
    states: &mut [VisitState],
) -> Option<usize> {
    match states[node_id] {
        VisitState::Unvisited => {}
        VisitState::Visiting | VisitState::Visited => return None,
    }

    states[node_id] = VisitState::Visiting;

    let node = &index.nodes[node_id];

    let cardinality = if node.is_leaf {
        if !node.children.is_empty() {
            return None;
        }

        node.cardinality
    } else {
        if node.children.is_empty() {
            return None;
        }

        let mut child_cardinality = 0usize;

        for child_id in &node.children {
            if *child_id >= index.nodes.len() {
                return None;
            }

            child_cardinality = child_cardinality
                .checked_add(subtree_leaf_cardinality(index, *child_id, states)?)?;
        }

        if child_cardinality != node.cardinality {
            return None;
        }

        child_cardinality
    };

    states[node_id] = VisitState::Visited;

    Some(cardinality)
}
