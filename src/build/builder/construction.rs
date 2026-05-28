//! Recursive index construction.

use crate::build::builder::acceptance::accepts_split_quality;
use crate::build::builder::config::BuildConfig;
use crate::build::builder::types::{AcceptedStructuralSplit, ValidatedFSEIndex};
use crate::build::splitter::best_structural_split;
use crate::build::validate_index;
use crate::math::Vector;
use crate::storage::{FSEIndex, PartitionNode};

/// Builder for constructing an FSE index from coordinate vectors.
///
/// # Runtime Role
///
/// `FSEBuilder` owns the construction configuration and recursively creates a
/// hierarchy of partition nodes.
///
/// # Formal Reference
///
/// This implements the construction pipeline:
///
/// 1. Compute local centroid.
/// 2. Compute bounded support.
/// 3. Encode residuals.
/// 4. Select a geometrically useful split.
/// 5. Recurse only when subdivision improves structural tightness, unless the
///    partition exceeds the hard leaf cardinality limit.
#[derive(Clone, Debug, PartialEq)]
pub struct FSEBuilder {
    config: BuildConfig,
}

impl FSEBuilder {
    /// Creates a builder from a build configuration.
    pub fn new(config: BuildConfig) -> Self {
        Self { config }
    }

    /// Returns the builder configuration.
    pub fn config(&self) -> &BuildConfig {
        &self.config
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

        let dimensions = points[0].dimensions();

        for point in points {
            assert_eq!(
                point.dimensions(),
                dimensions,
                "all points must have the same dimensionality"
            );
        }

        let mut nodes = Vec::new();
        let root = self.build_node(points.to_vec(), 0, &mut nodes);

        FSEIndex::new(nodes, root)
    }

    /// Builds an index and validates the constructed result.
    ///
    /// # Runtime Role
    ///
    /// This is a convenience method for callers that want both the constructed
    /// hierarchy and an immediate validation report.
    ///
    /// # Panics
    ///
    /// Panics under the same conditions as [`FSEBuilder::build`].
    pub fn build_validated(&self, points: &[Vector]) -> ValidatedFSEIndex {
        let index = self.build(points);
        let validation = validate_index(&index, self.config.max_leaf_size);

        ValidatedFSEIndex { index, validation }
    }

    fn build_node(
        &self,
        points: Vec<Vector>,
        depth: usize,
        nodes: &mut Vec<PartitionNode>,
    ) -> usize {
        let id = nodes.len();

        if self.should_stop_without_split(points.len(), depth) {
            let node = PartitionNode::from_points(id, &points);
            nodes.push(node);
            return id;
        }

        let Some(split) = self.accepted_structural_split(&points) else {
            // optional split didnt earn the extra node
            let node = PartitionNode::from_points(id, &points);
            nodes.push(node);
            return id;
        };

        if self.config.require_positive_split_volume_reduction && !split.was_forced {
            debug_assert!(
                accepts_split_quality(&split.metrics),
                "optional accepted split should improve bounding volume or degenerate extent"
            );
        }

        let placeholder = PartitionNode::internal_from_points(id, &points, Vec::new());
        nodes.push(placeholder);

        let left_id = self.build_node(split.left_points, depth + 1, nodes);
        let right_id = self.build_node(split.right_points, depth + 1, nodes);

        nodes[id].children = vec![left_id, right_id];

        id
    }

    fn should_stop_without_split(&self, point_count: usize, depth: usize) -> bool {
        point_count <= self.config.target_leaf_size || depth >= self.config.max_depth
    }

    fn should_force_split(&self, point_count: usize) -> bool {
        point_count > self.config.max_leaf_size
    }

    fn accepted_structural_split(&self, points: &[Vector]) -> Option<AcceptedStructuralSplit> {
        let split = best_structural_split(points);
        let was_forced = self.should_force_split(points.len());
        let metrics = split.score.metrics;

        if self.config.require_positive_split_volume_reduction
            && !was_forced
            && !accepts_split_quality(&metrics)
        {
            // geometry gate only applies while we are still under the hard cap
            return None;
        }

        Some(AcceptedStructuralSplit {
            left_points: split.left_points,
            right_points: split.right_points,
            metrics,
            was_forced,
        })
    }
}
