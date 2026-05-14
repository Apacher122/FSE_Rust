//! Benchmarking and baseline utilities.
//!
//! This module contains simple baseline implementations used to compare FSE
//! query behavior against conventional scan-based execution.

pub mod baseline;
pub mod comparison;
pub mod config;
pub mod datasets;
pub mod kd_tree;
pub mod pruning;
pub mod runner;
pub mod scan;
pub mod summary;
pub mod timing;
pub mod workload;

pub use baseline::{
    BaselineComparisonLabels, BaselineKind, BaselineQueryReport, BaselineQueryStats,
    BaselineRegistry, FlatScanBaseline, RangeQueryBaseline, execute_range_baseline,
};
pub use comparison::{
    QueryComparisonReport, compare_query_execution, compare_query_execution_repeated,
    compare_query_execution_with_baseline,
};
pub use config::{BenchmarkDatasetKind, BenchmarkSuiteConfig};
pub use datasets::{
    ClusteredDatasetConfig, clustered_points_2d, generate_clustered_points_2d,
    large_clustered_points_2d,
};
pub use kd_tree::KdTreeBaseline;
pub use pruning::{PruningEfficiencyReport, pruning_efficiency_report};
pub use runner::{
    BenchmarkSuiteReport, WorkloadPruningReport, run_benchmark_suite, run_benchmark_suite_repeated,
    run_benchmark_suite_with_registry,
};
pub use scan::{FlatScanReport, FlatScanStats, flat_scan, flat_scan_with_stats};
pub use summary::{
    AggregateWorkloadMetrics, WorkloadComparisonSummary, aggregate_workload_metrics,
    summarize_workload_comparisons,
};
pub use timing::{
    RepeatedComparisonTimingReport, RepeatedTimingConfig, RepeatedTimingReport, TimingReport,
    duration_ratio, measure_elapsed, measure_repeated,
};
pub use workload::{
    QueryWorkloadCase, RangeWorkloadConfig, clustered_workload_cases,
    generate_range_workload_cases, large_clustered_workload_cases,
};
