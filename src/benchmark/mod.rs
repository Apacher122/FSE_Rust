//! Benchmarking and baseline utilities.
//!
//! This module contains simple baseline implementations used to compare FSE
//! query behavior against conventional scan-based execution.

pub mod comparison;
pub mod datasets;
pub mod pruning;
pub mod runner;
pub mod scan;
pub mod summary;
pub mod timing;
pub mod workload;

pub use comparison::{QueryComparisonReport, compare_query_execution};
pub use datasets::clustered_points_2d;
pub use pruning::{PruningEfficiencyReport, pruning_efficiency_report};
pub use runner::{BenchmarkSuiteReport, WorkloadPruningReport, run_benchmark_suite};
pub use scan::{FlatScanReport, FlatScanStats, flat_scan, flat_scan_with_stats};
pub use summary::{
    AggregateWorkloadMetrics, WorkloadComparisonSummary, aggregate_workload_metrics,
    summarize_workload_comparisons,
};
pub use timing::{TimingReport, measure_elapsed};
pub use workload::{
    QueryWorkloadCase, RangeWorkloadConfig, clustered_workload_cases, generate_range_workload_cases,
};
