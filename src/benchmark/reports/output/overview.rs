//! Benchmark run overview rendering.

use std::fmt::Write;

use crate::build::{
    IndexFootprintComparisonMetrics, IndexFootprintMetrics, IndexStructureMetrics,
    IndexValidationReport,
};
use crate::query::QueryExecutionMode;

/// Header information printed before benchmark reports.
///
/// # Runtime Role
///
/// `BenchmarkRunOverview` keeps benchmark run metadata separate from the binary
/// entry point so terminal rendering can be reused and tested independently.
#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkRunOverview {
    /// Number of records in the selected dataset.
    pub dataset_records: usize,

    /// Number of nodes in the constructed FSE index.
    pub index_nodes: usize,

    /// Number of workload cases in the benchmark run.
    pub workloads: usize,

    /// Comma-separated baseline names selected for the run.
    pub baselines: String,

    /// Number of repeated timing iterations.
    pub timing_iterations: usize,

    /// Target FSE leaf size used during construction.
    pub target_leaf_size: usize,

    /// Maximum FSE leaf size used during construction.
    pub max_leaf_size: usize,

    /// Maximum FSE build depth used during construction.
    pub max_depth: usize,

    /// FSE query execution mode used by the benchmark run.
    pub fse_execution_mode: QueryExecutionMode,

    /// Minimum retained-leaf count required before parallel FSE mode uses Rayon.
    pub fse_parallel_min_retained_leaves: usize,

    /// Structural metrics for the constructed FSE index.
    pub index_structure: IndexStructureMetrics,

    /// Logical scalar footprint metrics for the constructed FSE index.
    pub index_footprint: IndexFootprintMetrics,

    /// Footprint comparison metrics for the constructed FSE index.
    pub index_footprint_comparison: IndexFootprintComparisonMetrics,

    /// Validation report for the constructed FSE index.
    pub validation: IndexValidationReport,
}

impl BenchmarkRunOverview {
    /// Returns the user-facing name for the configured FSE execution mode.
    pub fn fse_execution_mode_name(&self) -> &'static str {
        match self.fse_execution_mode {
            QueryExecutionMode::Serial => "serial",
            QueryExecutionMode::Parallel => "parallel",
        }
    }
}

/// Renders the benchmark run overview.
pub fn render_benchmark_overview(overview: &BenchmarkRunOverview) -> String {
    let mut output = String::new();

    writeln!(output, "FSE benchmark suite").unwrap();
    writeln!(output, "===================").unwrap();
    writeln!(output).unwrap();

    writeln!(output, "Dataset records: {}", overview.dataset_records).unwrap();
    writeln!(output, "Index nodes: {}", overview.index_nodes).unwrap();
    writeln!(
        output,
        "Leaf nodes: {}",
        overview.index_structure.leaf_count
    )
    .unwrap();
    writeln!(
        output,
        "Internal nodes: {}",
        overview.index_structure.internal_node_count
    )
    .unwrap();
    writeln!(output, "Workloads: {}", overview.workloads).unwrap();
    writeln!(output, "Baselines: {}", overview.baselines).unwrap();
    writeln!(output, "Timing iterations: {}", overview.timing_iterations).unwrap();
    writeln!(output, "Target leaf size: {}", overview.target_leaf_size).unwrap();
    writeln!(output, "Max leaf size: {}", overview.max_leaf_size).unwrap();
    writeln!(output, "Max build depth: {}", overview.max_depth).unwrap();
    writeln!(
        output,
        "Max leaf cardinality: {}",
        overview.index_structure.max_leaf_cardinality
    )
    .unwrap();
    writeln!(
        output,
        "Average leaf cardinality: {:.2}",
        overview.index_structure.average_leaf_cardinality
    )
    .unwrap();
    writeln!(
        output,
        "Total leaf volume: {:.2}",
        overview.index_structure.total_leaf_volume
    )
    .unwrap();
    writeln!(
        output,
        "Index density: {:.2}",
        overview.index_structure.index_density
    )
    .unwrap();
    writeln!(
        output,
        "Zero-volume leaves: {}",
        overview.index_structure.zero_volume_leaf_count
    )
    .unwrap();
    writeln!(
        output,
        "Encoded coordinate scalars: {}",
        overview.index_footprint.encoded_coordinate_scalar_count
    )
    .unwrap();
    writeln!(
        output,
        "Residual scalars: {}",
        overview.index_footprint.residual_scalar_count
    )
    .unwrap();
    writeln!(
        output,
        "Centroid scalars: {}",
        overview.index_footprint.centroid_scalar_count
    )
    .unwrap();
    writeln!(
        output,
        "Bounds scalars: {}",
        overview.index_footprint.bounds_scalar_count
    )
    .unwrap();
    writeln!(
        output,
        "Structural metadata scalars: {}",
        overview.index_footprint.structural_metadata_scalar_count
    )
    .unwrap();
    writeln!(
        output,
        "Total counted index scalars: {}",
        overview.index_footprint.total_index_scalar_count
    )
    .unwrap();
    writeln!(
        output,
        "Index-to-encoded scalar ratio: {:.2}",
        overview.index_footprint.index_to_encoded_scalar_ratio
    )
    .unwrap();
    writeln!(
        output,
        "Encoded baseline scalars: {}",
        overview
            .index_footprint_comparison
            .encoded_baseline_scalar_count
    )
    .unwrap();
    writeln!(
        output,
        "Scalar delta from encoded baseline: {}",
        overview
            .index_footprint_comparison
            .scalar_delta_from_baseline
    )
    .unwrap();
    writeln!(
        output,
        "Index-to-encoded baseline scalar ratio: {:.2}",
        overview
            .index_footprint_comparison
            .index_to_encoded_baseline_scalar_ratio
    )
    .unwrap();
    writeln!(
        output,
        "Structural metadata share of index: {:.2}",
        overview
            .index_footprint_comparison
            .structural_metadata_share_of_index
    )
    .unwrap();
    writeln!(
        output,
        "Index exceeds encoded baseline: {}",
        overview
            .index_footprint_comparison
            .index_exceeds_encoded_baseline
    )
    .unwrap();
    writeln!(
        output,
        "Structural metadata dominates residuals: {}",
        overview
            .index_footprint_comparison
            .structural_metadata_dominates_residuals
    )
    .unwrap();
    writeln!(
        output,
        "FSE execution: {}",
        overview.fse_execution_mode_name()
    )
    .unwrap();
    writeln!(
        output,
        "FSE parallel min leaves: {}",
        overview.fse_parallel_min_retained_leaves
    )
    .unwrap();
    writeln!(output).unwrap();

    writeln!(
        output,
        "Timing ratio meaning: baseline elapsed / FSE elapsed"
    )
    .unwrap();
    writeln!(output, "  above 1.0 means FSE measured faster for that run").unwrap();
    writeln!(output).unwrap();

    writeln!(
        output,
        "Index validation: {}",
        overview.validation.is_valid()
    )
    .unwrap();
    writeln!(
        output,
        "  node identifier consistency valid: {}",
        overview.validation.node_identifier_consistency_valid
    )
    .unwrap();
    writeln!(
        output,
        "  partition dimensional metadata valid: {}",
        overview.validation.partition_dimensional_metadata_valid
    )
    .unwrap();
    writeln!(
        output,
        "  leaf cardinality valid: {}",
        overview.validation.leaf_cardinality_valid
    )
    .unwrap();
    writeln!(
        output,
        "  leaf reconstruction metadata valid: {}",
        overview.validation.leaf_reconstruction_metadata_valid
    )
    .unwrap();
    writeln!(
        output,
        "  leaf record bounds valid: {}",
        overview.validation.leaf_record_bounds_valid
    )
    .unwrap();
    writeln!(
        output,
        "  leaf ownership cardinality valid: {}",
        overview.validation.leaf_ownership_cardinality_valid
    )
    .unwrap();
    writeln!(
        output,
        "  hierarchy topology valid: {}",
        overview.validation.hierarchy_topology_valid
    )
    .unwrap();
    writeln!(
        output,
        "  parent-child bounds valid: {}",
        overview.validation.parent_child_bounds_valid
    )
    .unwrap();
    writeln!(output).unwrap();

    output
}
