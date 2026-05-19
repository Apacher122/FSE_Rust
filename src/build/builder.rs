//! Recursive FSE index builder.

use crate::build::metrics::SplitQualityMetrics;
use crate::build::splitter::best_median_split;
use crate::build::{IndexValidationReport, validate_index};
use crate::math::Vector;
use crate::storage::{FSEIndex, PartitionNode};

/// Configuration for recursive FSE index construction.
#[derive(Clone, Debug, PartialEq)]
pub struct BuildConfig {
    /// Target number of records stored in a leaf partition.
    ///
    /// # Runtime Role
    ///
    /// The builder starts considering subdivision when a partition exceeds this
    /// target. If the partition is still below the hard maximum, geometric split
    /// quality can decide whether subdivision is worthwhile.
    pub target_leaf_size: usize,

    /// Hard maximum number of records allowed in a leaf partition.
    ///
    /// # Runtime Role
    ///
    /// This value is the validation limit. Partitions above this size are forced
    /// to split when depth still allows it, even if the candidate split does not
    /// improve bounding volume.
    pub max_leaf_size: usize,

    /// Maximum recursive depth allowed during construction.
    pub max_depth: usize,

    /// Whether optional recursive subdivision requires geometric improvement.
    ///
    /// # Runtime Role
    ///
    /// When enabled, the builder rejects optional splits whose child bounds do
    /// not improve the parent geometry. Normal partitions require volume
    /// improvement. Degenerate zero-volume partitions can use extent improvement
    /// as the fallback geometric signal.
    pub require_positive_split_volume_reduction: bool,
}

impl BuildConfig {
    /// Creates a new build configuration.
    ///
    /// # Runtime Role
    ///
    /// The default target leaf size is equal to the hard maximum leaf size. This
    /// preserves the previous constructor behavior while allowing callers to
    /// lower the target size later for optional density-aware refinement.
    ///
    /// # Panics
    ///
    /// Panics when `max_leaf_size` is zero.
    pub fn new(max_leaf_size: usize, max_depth: usize) -> Self {
        assert!(max_leaf_size > 0, "max_leaf_size must be greater than zero");

        Self {
            target_leaf_size: max_leaf_size,
            max_leaf_size,
            max_depth,
            require_positive_split_volume_reduction: true,
        }
    }

    /// Returns a copy of this configuration with a different target leaf size.
    ///
    /// # Runtime Role
    ///
    /// A target below the hard maximum lets the builder attempt optional
    /// density-aware refinement while still preserving the hard validation limit.
    ///
    /// # Panics
    ///
    /// Panics when `target_leaf_size` is zero or greater than `max_leaf_size`.
    pub fn with_target_leaf_size(mut self, target_leaf_size: usize) -> Self {
        assert!(
            target_leaf_size > 0,
            "target_leaf_size must be greater than zero"
        );
        assert!(
            target_leaf_size <= self.max_leaf_size,
            "target_leaf_size must not exceed max_leaf_size"
        );

        self.target_leaf_size = target_leaf_size;
        self
    }

    /// Returns a copy of this configuration with split-volume gating changed.
    ///
    /// # Runtime Role
    ///
    /// This is primarily useful for tests and controlled experiments. Production
    /// benchmarking should normally keep the geometry gate enabled so optional
    /// subdivision does not grow the hierarchy without a geometric reason.
    pub fn with_positive_split_volume_reduction_required(mut self, required: bool) -> Self {
        self.require_positive_split_volume_reduction = required;
        self
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            target_leaf_size: 64,
            max_leaf_size: 64,
            max_depth: 32,
            require_positive_split_volume_reduction: true,
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

        let Some(split) = self.accepted_median_split(&points) else {
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

    fn accepted_median_split(&self, points: &[Vector]) -> Option<AcceptedMedianSplit> {
        let split = best_median_split(points);
        let was_forced = self.should_force_split(points.len());
        let metrics = split.score.metrics;

        if self.config.require_positive_split_volume_reduction
            && !was_forced
            && !accepts_split_quality(&metrics)
        {
            // geometry gate only applies while we are still under the hard cap
            return None;
        }

        Some(AcceptedMedianSplit {
            left_points: split.left_points,
            right_points: split.right_points,
            metrics,
            was_forced,
        })
    }
}

/// Returns whether a split improves structural geometry.
///
/// # Runtime Role
///
/// The builder uses this as the optional split acceptance criterion. A split is
/// useful when it reduces combined child volume. If the parent volume is zero,
/// volume cannot be reduced, so extent reduction becomes the fallback signal.
pub fn accepts_split_quality(metrics: &SplitQualityMetrics) -> bool {
    if metrics.reduces_volume() {
        return true;
    }

    // skinny data still deserves a useful split
    metrics.parent_volume == 0.0 && metrics.reduces_extent()
}

#[derive(Clone, Debug, PartialEq)]
struct AcceptedMedianSplit {
    left_points: Vec<Vector>,
    right_points: Vec<Vector>,
    metrics: SplitQualityMetrics,
    was_forced: bool,
}
