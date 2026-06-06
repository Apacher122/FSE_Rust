//! Structural metric report types.

use crate::math::Scalar;

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

/// Logical scalar footprint metrics for an FSE index.
///
/// # Runtime Role
///
/// `IndexFootprintMetrics` counts coordinate-like scalar values stored in the
/// index representation. The counts distinguish encoded input coordinates,
/// residual values, and the geometric metadata used by query traversal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IndexFootprintMetrics {
    /// Dimensionality of the represented coordinate space.
    pub dimensions: usize,

    /// Number of logical records represented by the index.
    pub record_count: usize,

    /// Number of partition nodes in the index.
    pub node_count: usize,

    /// Number of leaf partitions in the index.
    pub leaf_count: usize,

    /// Number of scalar coordinates in the encoded input.
    pub encoded_coordinate_scalar_count: usize,

    /// Number of scalar residual values stored across all nodes.
    pub residual_scalar_count: usize,

    /// Number of scalar centroid values stored across all nodes.
    pub centroid_scalar_count: usize,

    /// Number of scalar bounding values stored across all nodes.
    pub bounds_scalar_count: usize,

    /// Number of scalar centroid and bounds values stored across all nodes.
    pub structural_metadata_scalar_count: usize,

    /// Total scalar footprint counted by these metrics.
    pub total_index_scalar_count: usize,

    /// Residual scalar count divided by encoded coordinate scalar count.
    pub residual_to_encoded_scalar_ratio: Scalar,

    /// Structural metadata scalar count divided by encoded coordinate scalar count.
    pub structural_to_encoded_scalar_ratio: Scalar,

    /// Total counted index scalar count divided by encoded coordinate scalar count.
    pub index_to_encoded_scalar_ratio: Scalar,
}

impl IndexFootprintMetrics {
    /// Returns true when the footprint has no represented records.
    pub fn is_empty(&self) -> bool {
        self.record_count == 0
    }
}

/// Comparison between an FSE footprint and an encoded coordinate baseline.
///
/// # Runtime Role
///
/// `IndexFootprintComparisonMetrics` derives storage-footprint interpretation
/// from [`IndexFootprintMetrics`]. The baseline is the scalar count required
/// to store the encoded coordinate matrix for the represented records.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IndexFootprintComparisonMetrics {
    /// Scalar count for the encoded coordinate baseline.
    pub encoded_baseline_scalar_count: usize,

    /// Total scalar count stored by the measured FSE index.
    pub index_scalar_count: usize,

    /// Signed scalar difference between the FSE index and encoded baseline.
    pub scalar_delta_from_baseline: i128,

    /// Number of scalar residual values stored by the FSE index.
    pub residual_scalar_count: usize,

    /// Number of scalar geometric metadata values stored by the FSE index.
    pub structural_metadata_scalar_count: usize,

    /// FSE index scalar count divided by encoded baseline scalar count.
    pub index_to_encoded_baseline_scalar_ratio: Scalar,

    /// Residual scalar count divided by encoded baseline scalar count.
    pub residual_to_encoded_baseline_scalar_ratio: Scalar,

    /// Structural metadata scalar count divided by encoded baseline scalar count.
    pub structural_metadata_to_encoded_baseline_scalar_ratio: Scalar,

    /// Structural metadata scalar count divided by total FSE index scalar count.
    pub structural_metadata_share_of_index: Scalar,

    /// Whether the FSE scalar count is greater than the encoded baseline count.
    pub index_exceeds_encoded_baseline: bool,

    /// Whether structural metadata is greater than residual storage.
    pub structural_metadata_dominates_residuals: bool,
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
