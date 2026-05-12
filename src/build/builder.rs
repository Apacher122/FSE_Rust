//! Recursive FSE index builder.

use crate::build::splitter::median_split;
use crate::build::{IndexValidationReport, validate_index};
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

impl BuildConfig {
    /// Creates a new build configuration.
    ///
    /// # Panics
    ///
    /// Panics when `max_leaf_size` is zero.
    pub fn new(max_leaf_size: usize, max_depth: usize) -> Self {
        assert!(max_leaf_size > 0, "max_leaf_size must be greater than zero");

        Self {
            max_leaf_size,
            max_depth,
        }
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            max_leaf_size: 64,
            max_depth: 32,
        }
    }
}

/// Builder output paired with validation results.
///
/// # Runtime Role
///
/// `ValidatedFSEIndex` is useful when construction should immediately report
/// whether the generated index satisfies core hierarchy invariants.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedFSEIndex {
    /// Constructed FSE index.
    pub index: FSEIndex,

    /// Validation report for the constructed index.
    pub validation: IndexValidationReport,
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

        let should_stop =
            points.len() <= self.config.max_leaf_size || depth >= self.config.max_depth;

        if should_stop {
            let node = PartitionNode::from_points(id, &points);
            nodes.push(node);
            return id;
        }

        let (left_points, right_points) = median_split(&points);

        let placeholder = PartitionNode::internal_from_points(id, &points, Vec::new());
        nodes.push(placeholder);

        let left_id = self.build_node(left_points, depth + 1, nodes);
        let right_id = self.build_node(right_points, depth + 1, nodes);

        nodes[id].children = vec![left_id, right_id];

        id
    }
}
