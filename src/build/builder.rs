//! Recursive FSE index builder.

use crate::build::splitter::median_split_on_axis;
use crate::math::Vector;
use crate::storage::{FSEIndex, PartitionNode};

/// Configuration for recursive FSE index construction.
#[derive(Clone, Debug, PartialEq)]
pub struct BuildConfig {
    /// Maximum number of records stored in a leaf partition.
    pub max_leaf_size: usize,
    /// Maximum recursive depth allowed during construction.
    pub max_depth: usize,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            max_leaf_size: 64,
            max_depth: 32,
        }
    }
}

/// Builder for constructing an FSE index from coordinate vectors.
///
/// # Runtime Role
///
/// `FSEBuilder` owns the construction configuration and recursively creates a
/// hierarchy of partition nodes.
///
/// # Formal Reference
///
/// This implements the initial construction pipeline corresponding to:
///
/// 1. Compute local centroid.
/// 2. Compute bounded support.
/// 3. Encode residuals.
/// 4. Split recursively until a stopping rule is reached.
pub struct FSEBuilder {
    config: BuildConfig,
}

impl FSEBuilder {
    /// Creates a builder from a build configuration.
    pub fn new(config: BuildConfig) -> Self {
        Self { config }
    }

    /// Builds an index from raw coordinate vectors.
    ///
    /// # Panics
    ///
    /// Panics when the point set is empty or dimensionality is inconsistent.
    pub fn build(&self, points: &[Vector]) -> FSEIndex {
        assert!(
            !points.is_empty(),
            "cannot build an index from empty points"
        );
        let mut nodes = Vec::new();
        let root = self.build_node(points.to_vec(), 0, &mut nodes);
        FSEIndex::new(nodes, root)
    }

    fn build_node(
        &self,
        points: Vec<Vector>,
        depth: usize,
        nodes: &mut Vec<PartitionNode>,
    ) -> usize {
        // FIXME: Recursive builder is clean, but could stack overflow if someone
        // passes a massive `max_depth` configuration.
        let id = nodes.len();
        let should_stop =
            points.len() <= self.config.max_leaf_size || depth >= self.config.max_depth;

        if should_stop {
            let node = PartitionNode::from_points(id, &points);
            nodes.push(node);
            return id;
        }

        // Just using dimension 0 for the prototype split axis selection logic
        let (left_points, right_points) = median_split_on_axis(&points, 0);

        let placeholder = PartitionNode::internal_from_points(id, &points, Vec::new());
        nodes.push(placeholder);

        let left_id = self.build_node(left_points, depth + 1, nodes);
        let right_id = self.build_node(right_points, depth + 1, nodes);

        nodes[id].children = vec![left_id, right_id];
        id
    }
}
