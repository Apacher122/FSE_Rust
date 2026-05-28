//! Leaf-cardinality validation.

use crate::storage::FSEIndex;

/// Validates that all leaf partitions satisfy the configured maximum leaf size.
///
/// # Runtime Role
///
/// This function is used to verify that recursive index construction respected
/// the configured leaf cardinality bound.
///
/// # Notes
///
/// Internal nodes are ignored because they may contain metadata and residuals
/// depending on the current storage layout. This validation only checks leaf
/// partitions.
pub fn validate_leaf_cardinality(index: &FSEIndex, max_leaf_size: usize) -> bool {
    index
        .nodes
        .iter()
        .filter(|node| node.is_leaf)
        .all(|node| node.cardinality <= max_leaf_size)
}
