//! Benchmarking and baseline utilities.
//!
//! This module contains simple baseline implementations used to compare FSE
//! query behavior against conventional scan-based execution.

pub mod baseline;
pub mod cli;
pub mod comparison;
pub mod config;
pub mod csv;
pub mod datasets;
pub mod kd_tree;
pub mod multi_summary;
pub mod pruning;
pub mod r_tree;
pub mod runner;
pub mod scan;
pub mod summary;
pub mod timing;
pub mod workload;

pub use baseline::{
    BaselineComparisonLabels, BaselineKind, BaselineQueryReport, BaselineQueryStats,
    BaselineRegistry, FlatScanBaseline, RangeQueryBaseline, execute_range_baseline,
};
pub use cli::{
    BenchmarkCliConfig, benchmark_usage, parse_benchmark_cli_config, parse_benchmark_config,
};
pub use comparison::{
    QueryComparisonReport, compare_query_execution, compare_query_execution_repeated,
    compare_query_execution_with_baseline,
};
pub use config::{BenchmarkDatasetKind, BenchmarkSuiteConfig};
pub use csv::multi_baseline_aggregate_summary_to_csv;
pub use datasets::{
    ClusteredDatasetConfig, clustered_points_2d, generate_clustered_points_2d,
    large_clustered_points_2d,
};
pub use kd_tree::KdTreeBaseline;
pub use multi_summary::{
    BaselineAggregateSummary, MultiBaselineAggregateSummary, summarize_multi_baseline_aggregates,
};
pub use pruning::{PruningEfficiencyReport, pruning_efficiency_report};
pub use r_tree::RTreeBaseline;
pub use runner::{
    BaselineBenchmarkSuiteReport, BenchmarkSuiteReport, MultiBaselineBenchmarkSuiteReport,
    WorkloadPruningReport, run_benchmark_suite, run_benchmark_suite_repeated,
    run_benchmark_suite_with_registry, run_multi_baseline_benchmark_suite,
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
