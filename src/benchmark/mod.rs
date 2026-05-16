//! Benchmarking and baseline utilities.
//!
//! This module contains baseline implementations, workload generators, benchmark
//! runners, and reporting utilities used to compare FSE query behavior against
//! conventional exact range-query execution paths.

pub mod app;
pub mod baselines;
pub mod cli;
pub mod config;
pub mod reports;
pub mod runner;
pub mod workloads;

pub use app::{
    BenchmarkApplicationContext, BenchmarkApplicationError, BenchmarkApplicationOutput,
    BenchmarkApplicationOutputWriter, BenchmarkApplicationRenderer,
    BenchmarkApplicationResultBundle, render_benchmark_application_terminal_output,
    run_benchmark_application,
};

pub use baselines::{
    BaselineComparisonLabels, BaselineKind, BaselineQueryReport, BaselineQueryStats,
    BaselineRegistry, BenchmarkBaselineSet, EXACT_RANGE_BASELINE_KINDS, FlatScanBaseline,
    FlatScanReport, FlatScanStats, KdTreeBaseline, RTreeBaseline, RangeQueryBaseline,
    baseline_kind_name_list, baseline_kind_names, exact_range_baseline_kinds,
    exact_range_baseline_vec, execute_range_baseline, flat_scan, flat_scan_with_stats,
    has_multiple_baselines,
};

pub use cli::{
    BenchmarkCliConfig, BenchmarkTerminalOutputMode, benchmark_usage, parse_benchmark_cli_config,
    parse_benchmark_config,
};

pub use config::{BenchmarkDatasetKind, BenchmarkSuiteConfig};

pub use reports::{
    AggregateWorkloadMetrics, BaselineAggregateSummary, BenchmarkCsvMetadata,
    BenchmarkCsvOutputConfig, BenchmarkCsvOutputKind, BenchmarkCsvWriteError,
    BenchmarkCsvWriteReport, BenchmarkRunOverview, MultiBaselineAggregateSummary,
    PruningEfficiencyReport, QueryComparisonReport, SelectivityBucket, SelectivityBucketSummary,
    SelectivityBucketedWorkloadSummary, WorkloadComparisonSummary, aggregate_workload_metrics,
    compare_points_lexicographically, compare_query_execution, compare_query_execution_repeated,
    compare_query_execution_with_baseline, duration_ratio, measure_elapsed, measure_repeated,
    multi_baseline_aggregate_summary_to_csv, multi_baseline_aggregate_summary_to_csv_with_metadata,
    multi_baseline_workload_report_to_csv, multi_baseline_workload_report_to_csv_with_metadata,
    pruning_efficiency_report, render_benchmark_overview, render_multi_baseline_summary,
    render_named_baseline_suite_report, render_selectivity_bucketed_workload_summary,
    render_suite_report, sort_points_lexicographically, summarize_multi_baseline_aggregates,
    summarize_workload_comparisons, summarize_workloads_by_selectivity,
    write_benchmark_csv_outputs, write_multi_baseline_aggregate_summary_csv,
    write_multi_baseline_aggregate_summary_csv_with_metadata,
    write_multi_baseline_workload_report_csv,
    write_multi_baseline_workload_report_csv_with_metadata,
};

pub use runner::{
    BaselineBenchmarkSuiteReport, BenchmarkSuiteReport, MultiBaselineBenchmarkSuiteReport,
    WorkloadPruningReport, run_benchmark_suite, run_benchmark_suite_repeated,
    run_benchmark_suite_with_registry, run_multi_baseline_benchmark_suite,
};

pub use workloads::{
    ClusteredDatasetConfig, QueryWorkloadCase, RangeWorkloadConfig, clustered_points_2d,
    clustered_workload_cases, generate_clustered_points_2d, generate_range_workload_cases,
    large_clustered_points_2d, large_clustered_workload_cases,
};
