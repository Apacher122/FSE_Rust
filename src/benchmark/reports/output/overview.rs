//! Benchmark run overview rendering.

use std::fmt::Write;

use crate::build::{IndexStructureMetrics, IndexValidationReport};
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
        "  leaf cardinality valid: {}",
        overview.validation.leaf_cardinality_valid
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
