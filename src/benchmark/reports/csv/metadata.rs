//! CSV run metadata.

use crate::benchmark::reports::output::BenchmarkRunOverview;
use crate::math::Scalar;

use super::document::format_scalar;

/// Metadata describing the benchmark run that produced a CSV export.
///
/// # Runtime Role
///
/// `BenchmarkCsvMetadata` makes exported benchmark rows self-describing so CSV
/// files can be compared later without relying on terminal output.
#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkCsvMetadata {
    /// Number of records in the selected dataset.
    pub dataset_records: usize,

    /// Number of nodes in the constructed FSE index.
    pub index_nodes: usize,

    /// Number of workload cases in the benchmark run.
    pub workload_count: usize,

    /// Comma-separated baseline names selected for the run.
    pub selected_baselines: String,

    /// Number of repeated timing iterations.
    pub timing_iterations: usize,

    /// Target FSE leaf size used during construction.
    pub target_leaf_size: usize,

    /// Maximum FSE leaf size used during construction.
    pub max_leaf_size: usize,

    /// Maximum FSE build depth used during construction.
    pub max_depth: usize,

    /// FSE query execution mode used by the benchmark run.
    pub fse_execution_mode: String,

    /// Minimum retained-leaf count required before parallel FSE mode uses Rayon.
    pub fse_parallel_min_retained_leaves: usize,

    /// Number of leaf partitions in the constructed index.
    pub index_leaf_count: usize,

    /// Number of internal nodes in the constructed index.
    pub index_internal_node_count: usize,

    /// Total cardinality across leaf partitions.
    pub index_total_leaf_cardinality: usize,

    /// Minimum leaf cardinality.
    pub index_min_leaf_cardinality: usize,

    /// Maximum leaf cardinality.
    pub index_max_leaf_cardinality: usize,

    /// Average leaf cardinality.
    pub index_average_leaf_cardinality: Scalar,

    /// Sum of leaf bounding volumes.
    pub index_total_leaf_volume: Scalar,

    /// Average leaf bounding volume.
    pub index_average_leaf_volume: Scalar,

    /// Aggregate structural density across leaves.
    pub index_density: Scalar,

    /// Number of zero-volume leaves.
    pub index_zero_volume_leaf_count: usize,

    /// Number of scalar coordinates in the encoded input.
    pub index_encoded_coordinate_scalar_count: usize,

    /// Number of scalar residual values stored across all nodes.
    pub index_residual_scalar_count: usize,

    /// Number of scalar centroid values stored across all nodes.
    pub index_centroid_scalar_count: usize,

    /// Number of scalar bounding values stored across all nodes.
    pub index_bounds_scalar_count: usize,

    /// Number of scalar centroid and bounds values stored across all nodes.
    pub index_structural_metadata_scalar_count: usize,

    /// Total scalar footprint counted for the constructed index.
    pub index_total_scalar_count: usize,

    /// Residual scalar count divided by encoded coordinate scalar count.
    pub index_residual_to_encoded_scalar_ratio: Scalar,

    /// Structural metadata scalar count divided by encoded coordinate scalar count.
    pub index_structural_to_encoded_scalar_ratio: Scalar,

    /// Total counted index scalar count divided by encoded coordinate scalar count.
    pub index_to_encoded_scalar_ratio: Scalar,

    /// Number of bytes used by one scalar value.
    pub index_scalar_size_bytes: usize,

    /// Estimated bytes for encoded coordinate scalar payloads.
    pub index_encoded_coordinate_estimated_bytes: usize,

    /// Estimated bytes for residual scalar payloads.
    pub index_residual_estimated_bytes: usize,

    /// Estimated bytes for centroid scalar payloads.
    pub index_centroid_estimated_bytes: usize,

    /// Estimated bytes for bounding scalar payloads.
    pub index_bounds_estimated_bytes: usize,

    /// Estimated bytes for centroid and bounds scalar payloads.
    pub index_structural_metadata_estimated_bytes: usize,

    /// Estimated bytes for residuals and structural metadata.
    pub index_total_estimated_bytes: usize,

    /// Scalar count for the encoded coordinate baseline.
    pub index_encoded_baseline_scalar_count: usize,

    /// Total scalar count stored by the measured FSE index.
    pub index_comparison_scalar_count: usize,

    /// Signed scalar difference between the FSE index and encoded baseline.
    pub index_scalar_delta_from_encoded_baseline: i128,

    /// FSE index scalar count divided by encoded baseline scalar count.
    pub index_to_encoded_baseline_scalar_ratio: Scalar,

    /// Residual scalar count divided by encoded baseline scalar count.
    pub index_residual_to_encoded_baseline_scalar_ratio: Scalar,

    /// Structural metadata scalar count divided by encoded baseline scalar count.
    pub index_structural_metadata_to_encoded_baseline_scalar_ratio: Scalar,

    /// Structural metadata scalar count divided by total FSE index scalar count.
    pub index_structural_metadata_share_of_index: Scalar,

    /// Whether the FSE scalar count is greater than the encoded baseline count.
    pub index_exceeds_encoded_baseline: bool,

    /// Whether structural metadata is greater than residual storage.
    pub index_structural_metadata_dominates_residuals: bool,

    /// Whether all index validation checks passed.
    pub index_valid: bool,

    /// Whether node identifiers match their index positions.
    pub node_identifier_consistency_valid: bool,

    /// Whether partition dimensional metadata validation passed.
    pub partition_dimensional_metadata_valid: bool,

    /// Whether leaf cardinality validation passed.
    pub leaf_cardinality_valid: bool,

    /// Whether leaf reconstruction metadata validation passed.
    pub leaf_reconstruction_metadata_valid: bool,

    /// Whether reconstructed leaf rows are contained by their leaf bounds.
    pub leaf_record_bounds_valid: bool,

    /// Whether leaf ownership cardinality validation passed.
    pub leaf_ownership_cardinality_valid: bool,

    /// Whether hierarchy topology validation passed.
    pub hierarchy_topology_valid: bool,

    /// Whether parent-child bounds validation passed.
    pub parent_child_bounds_valid: bool,
}

impl BenchmarkCsvMetadata {
    /// Builds CSV metadata from the benchmark overview used for terminal output.
    pub fn from_overview(overview: &BenchmarkRunOverview) -> Self {
        Self {
            dataset_records: overview.dataset_records,
            index_nodes: overview.index_nodes,
            workload_count: overview.workloads,
            selected_baselines: overview.baselines.clone(),
            timing_iterations: overview.timing_iterations,
            target_leaf_size: overview.target_leaf_size,
            max_leaf_size: overview.max_leaf_size,
            max_depth: overview.max_depth,
            fse_execution_mode: overview.fse_execution_mode_name().to_string(),
            fse_parallel_min_retained_leaves: overview.fse_parallel_min_retained_leaves,
            index_leaf_count: overview.index_structure.leaf_count,
            index_internal_node_count: overview.index_structure.internal_node_count,
            index_total_leaf_cardinality: overview.index_structure.total_leaf_cardinality,
            index_min_leaf_cardinality: overview.index_structure.min_leaf_cardinality,
            index_max_leaf_cardinality: overview.index_structure.max_leaf_cardinality,
            index_average_leaf_cardinality: overview.index_structure.average_leaf_cardinality,
            index_total_leaf_volume: overview.index_structure.total_leaf_volume,
            index_average_leaf_volume: overview.index_structure.average_leaf_volume,
            index_density: overview.index_structure.index_density,
            index_zero_volume_leaf_count: overview.index_structure.zero_volume_leaf_count,
            index_encoded_coordinate_scalar_count: overview
                .index_footprint
                .encoded_coordinate_scalar_count,
            index_residual_scalar_count: overview.index_footprint.residual_scalar_count,
            index_centroid_scalar_count: overview.index_footprint.centroid_scalar_count,
            index_bounds_scalar_count: overview.index_footprint.bounds_scalar_count,
            index_structural_metadata_scalar_count: overview
                .index_footprint
                .structural_metadata_scalar_count,
            index_total_scalar_count: overview.index_footprint.total_index_scalar_count,
            index_residual_to_encoded_scalar_ratio: overview
                .index_footprint
                .residual_to_encoded_scalar_ratio,
            index_structural_to_encoded_scalar_ratio: overview
                .index_footprint
                .structural_to_encoded_scalar_ratio,
            index_to_encoded_scalar_ratio: overview.index_footprint.index_to_encoded_scalar_ratio,
            index_scalar_size_bytes: overview.index_footprint_byte_estimates.scalar_size_bytes,
            index_encoded_coordinate_estimated_bytes: overview
                .index_footprint_byte_estimates
                .encoded_coordinate_bytes,
            index_residual_estimated_bytes: overview.index_footprint_byte_estimates.residual_bytes,
            index_centroid_estimated_bytes: overview.index_footprint_byte_estimates.centroid_bytes,
            index_bounds_estimated_bytes: overview.index_footprint_byte_estimates.bounds_bytes,
            index_structural_metadata_estimated_bytes: overview
                .index_footprint_byte_estimates
                .structural_metadata_bytes,
            index_total_estimated_bytes: overview.index_footprint_byte_estimates.total_index_bytes,
            index_encoded_baseline_scalar_count: overview
                .index_footprint_comparison
                .encoded_baseline_scalar_count,
            index_comparison_scalar_count: overview.index_footprint_comparison.index_scalar_count,
            index_scalar_delta_from_encoded_baseline: overview
                .index_footprint_comparison
                .scalar_delta_from_baseline,
            index_to_encoded_baseline_scalar_ratio: overview
                .index_footprint_comparison
                .index_to_encoded_baseline_scalar_ratio,
            index_residual_to_encoded_baseline_scalar_ratio: overview
                .index_footprint_comparison
                .residual_to_encoded_baseline_scalar_ratio,
            index_structural_metadata_to_encoded_baseline_scalar_ratio: overview
                .index_footprint_comparison
                .structural_metadata_to_encoded_baseline_scalar_ratio,
            index_structural_metadata_share_of_index: overview
                .index_footprint_comparison
                .structural_metadata_share_of_index,
            index_exceeds_encoded_baseline: overview
                .index_footprint_comparison
                .index_exceeds_encoded_baseline,
            index_structural_metadata_dominates_residuals: overview
                .index_footprint_comparison
                .structural_metadata_dominates_residuals,
            index_valid: overview.validation.is_valid(),
            node_identifier_consistency_valid: overview
                .validation
                .node_identifier_consistency_valid,
            partition_dimensional_metadata_valid: overview
                .validation
                .partition_dimensional_metadata_valid,
            leaf_cardinality_valid: overview.validation.leaf_cardinality_valid,
            leaf_reconstruction_metadata_valid: overview
                .validation
                .leaf_reconstruction_metadata_valid,
            leaf_record_bounds_valid: overview.validation.leaf_record_bounds_valid,
            leaf_ownership_cardinality_valid: overview.validation.leaf_ownership_cardinality_valid,
            hierarchy_topology_valid: overview.validation.hierarchy_topology_valid,
            parent_child_bounds_valid: overview.validation.parent_child_bounds_valid,
        }
    }
}

pub(super) fn metadata_header_fields() -> Vec<&'static str> {
    vec![
        "dataset_records",
        "index_nodes",
        "run_workload_count",
        "selected_baselines",
        "timing_iterations",
        "target_leaf_size",
        "max_leaf_size",
        "max_depth",
        "fse_execution_mode",
        "fse_parallel_min_retained_leaves",
        "index_leaf_count",
        "index_internal_node_count",
        "index_total_leaf_cardinality",
        "index_min_leaf_cardinality",
        "index_max_leaf_cardinality",
        "index_average_leaf_cardinality",
        "index_total_leaf_volume",
        "index_average_leaf_volume",
        "index_density",
        "index_zero_volume_leaf_count",
        "index_encoded_coordinate_scalar_count",
        "index_residual_scalar_count",
        "index_centroid_scalar_count",
        "index_bounds_scalar_count",
        "index_structural_metadata_scalar_count",
        "index_total_scalar_count",
        "index_residual_to_encoded_scalar_ratio",
        "index_structural_to_encoded_scalar_ratio",
        "index_to_encoded_scalar_ratio",
        "index_scalar_size_bytes",
        "index_encoded_coordinate_estimated_bytes",
        "index_residual_estimated_bytes",
        "index_centroid_estimated_bytes",
        "index_bounds_estimated_bytes",
        "index_structural_metadata_estimated_bytes",
        "index_total_estimated_bytes",
        "index_encoded_baseline_scalar_count",
        "index_comparison_scalar_count",
        "index_scalar_delta_from_encoded_baseline",
        "index_to_encoded_baseline_scalar_ratio",
        "index_residual_to_encoded_baseline_scalar_ratio",
        "index_structural_metadata_to_encoded_baseline_scalar_ratio",
        "index_structural_metadata_share_of_index",
        "index_exceeds_encoded_baseline",
        "index_structural_metadata_dominates_residuals",
        "index_valid",
        "node_identifier_consistency_valid",
        "partition_dimensional_metadata_valid",
        "leaf_cardinality_valid",
        "leaf_reconstruction_metadata_valid",
        "leaf_record_bounds_valid",
        "leaf_ownership_cardinality_valid",
        "hierarchy_topology_valid",
        "parent_child_bounds_valid",
    ]
}

pub(super) fn metadata_value_fields(metadata: &BenchmarkCsvMetadata) -> Vec<String> {
    vec![
        metadata.dataset_records.to_string(),
        metadata.index_nodes.to_string(),
        metadata.workload_count.to_string(),
        metadata.selected_baselines.clone(),
        metadata.timing_iterations.to_string(),
        metadata.target_leaf_size.to_string(),
        metadata.max_leaf_size.to_string(),
        metadata.max_depth.to_string(),
        metadata.fse_execution_mode.clone(),
        metadata.fse_parallel_min_retained_leaves.to_string(),
        metadata.index_leaf_count.to_string(),
        metadata.index_internal_node_count.to_string(),
        metadata.index_total_leaf_cardinality.to_string(),
        metadata.index_min_leaf_cardinality.to_string(),
        metadata.index_max_leaf_cardinality.to_string(),
        format_scalar(metadata.index_average_leaf_cardinality),
        format_scalar(metadata.index_total_leaf_volume),
        format_scalar(metadata.index_average_leaf_volume),
        format_scalar(metadata.index_density),
        metadata.index_zero_volume_leaf_count.to_string(),
        metadata.index_encoded_coordinate_scalar_count.to_string(),
        metadata.index_residual_scalar_count.to_string(),
        metadata.index_centroid_scalar_count.to_string(),
        metadata.index_bounds_scalar_count.to_string(),
        metadata.index_structural_metadata_scalar_count.to_string(),
        metadata.index_total_scalar_count.to_string(),
        format_scalar(metadata.index_residual_to_encoded_scalar_ratio),
        format_scalar(metadata.index_structural_to_encoded_scalar_ratio),
        format_scalar(metadata.index_to_encoded_scalar_ratio),
        metadata.index_scalar_size_bytes.to_string(),
        metadata
            .index_encoded_coordinate_estimated_bytes
            .to_string(),
        metadata.index_residual_estimated_bytes.to_string(),
        metadata.index_centroid_estimated_bytes.to_string(),
        metadata.index_bounds_estimated_bytes.to_string(),
        metadata
            .index_structural_metadata_estimated_bytes
            .to_string(),
        metadata.index_total_estimated_bytes.to_string(),
        metadata.index_encoded_baseline_scalar_count.to_string(),
        metadata.index_comparison_scalar_count.to_string(),
        metadata
            .index_scalar_delta_from_encoded_baseline
            .to_string(),
        format_scalar(metadata.index_to_encoded_baseline_scalar_ratio),
        format_scalar(metadata.index_residual_to_encoded_baseline_scalar_ratio),
        format_scalar(metadata.index_structural_metadata_to_encoded_baseline_scalar_ratio),
        format_scalar(metadata.index_structural_metadata_share_of_index),
        metadata.index_exceeds_encoded_baseline.to_string(),
        metadata
            .index_structural_metadata_dominates_residuals
            .to_string(),
        metadata.index_valid.to_string(),
        metadata.node_identifier_consistency_valid.to_string(),
        metadata.partition_dimensional_metadata_valid.to_string(),
        metadata.leaf_cardinality_valid.to_string(),
        metadata.leaf_reconstruction_metadata_valid.to_string(),
        metadata.leaf_record_bounds_valid.to_string(),
        metadata.leaf_ownership_cardinality_valid.to_string(),
        metadata.hierarchy_topology_valid.to_string(),
        metadata.parent_child_bounds_valid.to_string(),
    ]
}
