//! Structural metrics for partition quality.

use crate::build::splitter::median_split_on_axis;
use crate::math::{BoundingBox, Scalar, Vector};
use crate::storage::{FSEIndex, PartitionNode};

/// Metrics describing the geometric quality of one split.
///
/// # Runtime Role
///
/// `SplitQualityMetrics` quantifies whether a proposed split improves the
/// geometric tightness of a partition. The primary signal is combined child
/// bounding volume relative to parent bounding volume.
///
/// # Formal Reference
///
/// These values estimate the structural tightness objective used by FSE
/// partitioning. Tighter child support regions reduce geometric false positives
/// during metadata traversal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitQualityMetrics {
    /// Parent bounding volume before the split.
    pub parent_volume: Scalar,

    /// Sum of child bounding volumes after the split.
    pub combined_child_volume: Scalar,

    /// Relative reduction from parent volume to combined child volume.
    pub volume_reduction_ratio: Scalar,

    /// Sum of parent bounding extents across dimensions.
    pub parent_extent: Scalar,

    /// Sum of child bounding extents across dimensions.
    pub combined_child_extent: Scalar,

    /// Relative reduction from parent extent to combined child extent.
    pub extent_reduction_ratio: Scalar,

    /// Number of records in the parent partition.
    pub parent_cardinality: usize,

    /// Number of records in the left child partition.
    pub left_cardinality: usize,

    /// Number of records in the right child partition.
    pub right_cardinality: usize,

    /// Absolute difference between child cardinalities.
    pub balance_penalty: usize,
}

impl SplitQualityMetrics {
    /// Returns true when the split reduces combined child bounding volume.
    pub fn reduces_volume(&self) -> bool {
        self.combined_child_volume < self.parent_volume
    }

    /// Returns true when the split reduces combined child bounding extent.
    pub fn reduces_extent(&self) -> bool {
        self.combined_child_extent < self.parent_extent
    }

    /// Returns true when both children have equal cardinality.
    pub fn is_balanced(&self) -> bool {
        self.balance_penalty == 0
    }
}

/// Aggregate structural metrics for an FSE index.
///
/// # Runtime Role
///
/// `IndexStructureMetrics` summarizes the physical hierarchy produced by the
/// builder. These values make it possible to connect build policy choices to
/// query pruning behavior and reconstruction cost.
///
/// # Formal Reference
///
/// These metrics approximate structural density and bounding efficiency across
/// the leaf support regions used by query traversal and deferred reconstruction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IndexStructureMetrics {
    /// Total number of nodes in the index.
    pub node_count: usize,

    /// Number of leaf partitions.
    pub leaf_count: usize,

    /// Number of internal hierarchy nodes.
    pub internal_node_count: usize,

    /// Total number of records stored across leaf partitions.
    pub total_leaf_cardinality: usize,

    /// Smallest leaf cardinality.
    pub min_leaf_cardinality: usize,

    /// Largest leaf cardinality.
    pub max_leaf_cardinality: usize,

    /// Average number of records per leaf.
    pub average_leaf_cardinality: Scalar,

    /// Sum of all leaf bounding volumes.
    pub total_leaf_volume: Scalar,

    /// Average leaf bounding volume.
    pub average_leaf_volume: Scalar,

    /// Structural density across leaf partitions.
    pub index_density: Scalar,

    /// Number of leaves with zero bounding volume.
    pub zero_volume_leaf_count: usize,
}

impl IndexStructureMetrics {
    /// Returns true when the index has no leaf partitions.
    pub fn is_empty(&self) -> bool {
        self.leaf_count == 0
    }
}

/// Sibling-overlap metrics for an FSE hierarchy.
///
/// # Runtime Role
///
/// `SiblingOverlapMetrics` summarizes how much child bounding geometry overlaps
/// inside internal nodes. Overlap between siblings can increase retained
/// partitions because a query can intersect more than one child for the same
/// local region.
///
/// # Formal Reference
///
/// These metrics approximate the sibling-level over-approximation pressure that
/// affects Stage I geometric traversal.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SiblingOverlapMetrics {
    /// Number of sibling child-bound pairs inspected.
    pub sibling_pair_count: usize,

    /// Number of sibling pairs with positive overlap extent.
    pub overlapping_sibling_pair_count: usize,

    /// Sum of overlap extents across all sibling pairs.
    pub total_overlap_extent: Scalar,

    /// Average overlap extent per sibling pair.
    pub average_overlap_extent: Scalar,
}

impl SiblingOverlapMetrics {
    /// Returns true when no sibling pairs were measured.
    pub fn is_empty(&self) -> bool {
        self.sibling_pair_count == 0
    }

    /// Returns true when at least one sibling pair overlaps.
    pub fn has_overlap(&self) -> bool {
        self.overlapping_sibling_pair_count > 0
    }
}

/// Computes the structural density of a partition.
///
/// # Runtime Role
///
/// Structural density measures record concentration relative to admissible
/// bounding volume.
///
/// # Formal Reference
///
/// This implements `delta(P_k) = |D_k| / Vol(B_k)`.
///
/// # Notes
///
/// If the bounding volume is zero, this returns positive infinity for non-empty
/// partitions and zero for empty partitions.
pub fn partition_density(node: &PartitionNode) -> Scalar {
    let volume = node.bounds.volume();
    if volume == 0.0 {
        return if node.cardinality == 0 {
            0.0
        } else {
            Scalar::INFINITY
        };
    }

    node.cardinality as Scalar / volume
}

/// Computes aggregate structural density across leaf nodes.
///
/// # Runtime Role
///
/// Global density estimates geometric efficiency over the physical query leaves
/// that store reconstructable residual rows.
///
/// # Formal Reference
///
/// This implements `delta(F) = N / sum Vol(B_k)` over leaf partitions.
pub fn index_density(index: &FSEIndex) -> Scalar {
    index_structure_metrics(index).index_density
}

/// Computes aggregate structural metrics for an FSE index.
///
/// # Runtime Role
///
/// This function summarizes the hierarchy shape produced by the builder. The
/// benchmark layer uses it to explain whether build-policy changes are creating
/// tighter leaves or simply adding traversal nodes.
///
/// # Formal Reference
///
/// Since query cost depends on traversal work and reconstructed records, these
/// structural metrics provide the bridge between construction policy and query
/// execution behavior.
pub fn index_structure_metrics(index: &FSEIndex) -> IndexStructureMetrics {
    let leaves: Vec<&PartitionNode> = index.nodes.iter().filter(|node| node.is_leaf).collect();

    let leaf_count = leaves.len();
    let internal_node_count = index.nodes.len().saturating_sub(leaf_count);
    let total_leaf_cardinality: usize = leaves.iter().map(|node| node.cardinality).sum();
    let min_leaf_cardinality = leaves
        .iter()
        .map(|node| node.cardinality)
        .min()
        .unwrap_or(0);
    let max_leaf_cardinality = leaves
        .iter()
        .map(|node| node.cardinality)
        .max()
        .unwrap_or(0);

    let total_leaf_volume: Scalar = leaves.iter().map(|node| node.bounds.volume()).sum();
    let zero_volume_leaf_count = leaves
        .iter()
        .filter(|node| node.bounds.volume() == 0.0)
        .count();

    let average_leaf_cardinality = if leaf_count == 0 {
        0.0
    } else {
        total_leaf_cardinality as Scalar / leaf_count as Scalar
    };

    let average_leaf_volume = if leaf_count == 0 {
        0.0
    } else {
        total_leaf_volume / leaf_count as Scalar
    };

    let index_density = if total_leaf_volume == 0.0 {
        if total_leaf_cardinality == 0 {
            0.0
        } else {
            Scalar::INFINITY
        }
    } else {
        total_leaf_cardinality as Scalar / total_leaf_volume
    };

    // this is the build shape signal no more guessing
    IndexStructureMetrics {
        node_count: index.nodes.len(),
        leaf_count,
        internal_node_count,
        total_leaf_cardinality,
        min_leaf_cardinality,
        max_leaf_cardinality,
        average_leaf_cardinality,
        total_leaf_volume,
        average_leaf_volume,
        index_density,
        zero_volume_leaf_count,
    }
}

/// Computes sibling-overlap metrics for an FSE hierarchy.
///
/// # Runtime Role
///
/// This diagnostic helper walks internal hierarchy nodes and measures the
/// overlap extent between sibling child bounds. It does not affect split
/// selection or query execution.
///
/// # Formal Reference
///
/// Sibling overlap estimates one source of geometric over-retention during
/// Stage I traversal.
pub fn sibling_overlap_metrics(index: &FSEIndex) -> SiblingOverlapMetrics {
    let mut sibling_pair_count = 0;
    let mut overlapping_sibling_pair_count = 0;
    let mut total_overlap_extent = 0.0;

    for node in &index.nodes {
        if node.is_leaf || node.children.len() < 2 {
            continue;
        }

        for left_child_offset in 0..node.children.len() {
            for right_child_offset in (left_child_offset + 1)..node.children.len() {
                let left_child_id = node.children[left_child_offset];
                let right_child_id = node.children[right_child_offset];

                let left_bounds = &index.nodes[left_child_id].bounds;
                let right_bounds = &index.nodes[right_child_id].bounds;
                let overlap_extent = sibling_overlap_extent_sum(left_bounds, right_bounds);

                sibling_pair_count += 1;
                total_overlap_extent += overlap_extent;

                if overlap_extent > 0.0 {
                    overlapping_sibling_pair_count += 1;
                }
            }
        }
    }

    let average_overlap_extent = if sibling_pair_count == 0 {
        0.0
    } else {
        total_overlap_extent / sibling_pair_count as Scalar
    };

    SiblingOverlapMetrics {
        sibling_pair_count,
        overlapping_sibling_pair_count,
        total_overlap_extent,
        average_overlap_extent,
    }
}

/// Computes summed overlap extent between two sibling bounds.
///
/// # Runtime Role
///
/// The returned value is zero when bounds are disjoint in any dimension.
/// Otherwise, it sums the overlapping width along each dimension. This is a
/// lightweight pressure metric rather than a volume metric, so it still has a
/// useful signal when one dimension is degenerate.
///
/// # Panics
///
/// Panics when bounding dimensionality is inconsistent.
pub fn sibling_overlap_extent_sum(left_bounds: &BoundingBox, right_bounds: &BoundingBox) -> Scalar {
    assert_eq!(
        left_bounds.dimensions(),
        right_bounds.dimensions(),
        "sibling bounds must have matching dimensionality"
    );

    bounds_overlap_extent_sum_prevalidated(left_bounds, right_bounds)
}

/// Computes summed overlap extent for already-compatible bounding boxes.
///
/// # Runtime Role
///
/// This is the shared implementation for sibling-overlap diagnostics and split
/// scoring. Callers own the release-mode dimensionality contract so hot split
/// scoring can keep its existing debug-only validation behavior.
pub(crate) fn bounds_overlap_extent_sum_prevalidated(
    left_bounds: &BoundingBox,
    right_bounds: &BoundingBox,
) -> Scalar {
    debug_assert_eq!(
        left_bounds.dimensions(),
        right_bounds.dimensions(),
        "bounds should have matching dimensionality"
    );

    let mut overlap_extent = 0.0;

    for dimension in 0..left_bounds.dimensions() {
        let overlap_min = left_bounds.min[dimension].max(right_bounds.min[dimension]);
        let overlap_max = left_bounds.max[dimension].min(right_bounds.max[dimension]);
        let overlap_width = overlap_max - overlap_min;

        if overlap_width < 0.0 {
            return 0.0;
        }

        overlap_extent += overlap_width;
    }

    overlap_extent
}

/// Computes split quality metrics from parent and child point sets.
///
/// # Runtime Role
///
/// This helper evaluates the geometric effect of a split after the child point
/// sets are known. It is useful for testing split policies and for future build
/// heuristics that need to compare candidate subdivisions.
///
/// # Panics
///
/// Panics when any point set is empty, when dimensionality is inconsistent, or
/// when child cardinalities do not add up to parent cardinality.
pub fn split_quality_metrics(
    parent_points: &[Vector],
    left_points: &[Vector],
    right_points: &[Vector],
) -> SplitQualityMetrics {
    validate_split_point_sets(parent_points, left_points, right_points);

    let parent_bounds = BoundingBox::from_points(parent_points);
    let left_bounds = BoundingBox::from_points(left_points);
    let right_bounds = BoundingBox::from_points(right_points);

    split_quality_metrics_from_bounds(
        &parent_bounds,
        &left_bounds,
        &right_bounds,
        parent_points.len(),
        left_points.len(),
        right_points.len(),
    )
}

/// Computes split quality metrics for a median split along one axis.
///
/// # Runtime Role
///
/// This helper measures the child-volume effect of one candidate median split
/// axis without changing the builder. It is intended for split heuristic tests
/// and future build tuning.
///
/// # Panics
///
/// Panics when fewer than two points are provided, when the split dimension is
/// out of range, or when dimensionality is inconsistent.
pub fn split_quality_metrics_for_axis(
    points: &[Vector],
    split_dimension: usize,
) -> SplitQualityMetrics {
    let (left, right) = median_split_on_axis(points, split_dimension);

    // tiny helper but this is the metric we actually care about
    split_quality_metrics(points, &left, &right)
}

/// Computes split quality metrics from already computed bounds.
///
/// # Runtime Role
///
/// This helper avoids recomputing bounding boxes when callers already have the
/// parent and child support regions available.
///
/// # Panics
///
/// Panics when bounding dimensionality is inconsistent or when child
/// cardinalities do not add up to parent cardinality.
pub fn split_quality_metrics_from_bounds(
    parent_bounds: &BoundingBox,
    left_bounds: &BoundingBox,
    right_bounds: &BoundingBox,
    parent_cardinality: usize,
    left_cardinality: usize,
    right_cardinality: usize,
) -> SplitQualityMetrics {
    assert_eq!(
        parent_bounds.dimensions(),
        left_bounds.dimensions(),
        "left child bounds must match parent dimensionality"
    );
    assert_eq!(
        parent_bounds.dimensions(),
        right_bounds.dimensions(),
        "right child bounds must match parent dimensionality"
    );
    assert_eq!(
        left_cardinality + right_cardinality,
        parent_cardinality,
        "child cardinalities must add up to parent cardinality"
    );

    let parent_volume = parent_bounds.volume();
    let combined_child_volume = left_bounds.volume() + right_bounds.volume();
    let volume_reduction_ratio = reduction_ratio(parent_volume, combined_child_volume);

    let parent_extent = bounding_extent_sum(parent_bounds);
    let combined_child_extent =
        bounding_extent_sum(left_bounds) + bounding_extent_sum(right_bounds);
    let extent_reduction_ratio = reduction_ratio(parent_extent, combined_child_extent);

    let balance_penalty = left_cardinality.abs_diff(right_cardinality);

    SplitQualityMetrics {
        parent_volume,
        combined_child_volume,
        volume_reduction_ratio,
        parent_extent,
        combined_child_extent,
        extent_reduction_ratio,
        parent_cardinality,
        left_cardinality,
        right_cardinality,
        balance_penalty,
    }
}

/// Returns the sum of bounding widths across dimensions.
///
/// # Runtime Role
///
/// Extent is a fallback quality signal when volume collapses to zero because one
/// or more dimensions are degenerate.
pub fn bounding_extent_sum(bounds: &BoundingBox) -> Scalar {
    bounds
        .min
        .iter()
        .zip(&bounds.max)
        .map(|(minimum, maximum)| (maximum - minimum).max(0.0))
        .sum()
}

fn reduction_ratio(parent_value: Scalar, child_value: Scalar) -> Scalar {
    if parent_value <= 0.0 {
        return 0.0;
    }

    // can go negative when child boxes overlap too much
    (parent_value - child_value) / parent_value
}

fn validate_split_point_sets(
    parent_points: &[Vector],
    left_points: &[Vector],
    right_points: &[Vector],
) {
    assert!(
        !parent_points.is_empty(),
        "parent point set must not be empty"
    );
    assert!(!left_points.is_empty(), "left point set must not be empty");
    assert!(
        !right_points.is_empty(),
        "right point set must not be empty"
    );
    assert_eq!(
        left_points.len() + right_points.len(),
        parent_points.len(),
        "child point counts must add up to parent point count"
    );

    let dimensions = parent_points[0].dimensions();
    assert!(dimensions > 0, "points must have at least one dimension");

    // boring but worth catching before metrics lie to us
    for point in parent_points.iter().chain(left_points).chain(right_points) {
        assert_eq!(
            point.dimensions(),
            dimensions,
            "all split metric points must have the same dimensionality"
        );
    }
}
