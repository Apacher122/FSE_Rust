//! Benchmark reporting and measurement utilities.
//!
//! This module contains comparison reports, aggregate summaries, timing helpers,
//! pruning reports, terminal output rendering, CSV export utilities, and shared
//! result ordering helpers.

pub mod comparison;
pub mod csv;
pub mod csv_output;
pub mod multi_summary;
pub mod ordering;
pub mod output;
pub mod pruning;
pub mod selectivity;
pub mod summary;
pub mod timing;

pub use comparison::{
    QueryComparisonReport, TypedArchiveAppendDeltaMaintenanceTimingReport,
    TypedArchiveAppendRebuildTimingReport, TypedArchiveCompactionTimingReport,
    TypedArchiveLoadTimingError, TypedArchiveLoadTimingReport, TypedArchiveMaintenanceTimingReport,
    TypedQueryComparisonReport, compare_query_execution, compare_query_execution_repeated,
    compare_query_execution_repeated_with_options, compare_query_execution_with_baseline,
    compare_query_execution_with_baseline_and_options, compare_query_execution_with_options,
    compare_typed_archive_append_delta_maintenance_execution_repeated,
    compare_typed_archive_append_rebuild_execution_repeated,
    compare_typed_archive_compaction_execution_repeated, compare_typed_archive_load_execution,
    compare_typed_archive_load_execution_repeated,
    compare_typed_archive_maintenance_execution_repeated, compare_typed_query_execution,
    compare_typed_query_execution_repeated,
};
pub use csv::{
    BenchmarkCsvMetadata, BenchmarkCsvOutputConfig, multi_baseline_aggregate_summary_to_csv,
    multi_baseline_aggregate_summary_to_csv_with_metadata,
    multi_baseline_low_selectivity_gap_to_csv,
    multi_baseline_low_selectivity_gap_to_csv_with_metadata, multi_baseline_workload_report_to_csv,
    multi_baseline_workload_report_to_csv_with_metadata,
    write_multi_baseline_aggregate_summary_csv,
    write_multi_baseline_aggregate_summary_csv_with_metadata,
    write_multi_baseline_low_selectivity_gap_csv,
    write_multi_baseline_low_selectivity_gap_csv_with_metadata,
    write_multi_baseline_workload_report_csv,
    write_multi_baseline_workload_report_csv_with_metadata,
};
pub use csv_output::{
    BenchmarkCsvOutputKind, BenchmarkCsvWriteError, BenchmarkCsvWriteReport,
    write_benchmark_csv_outputs,
};
pub use multi_summary::{
    BaselineAggregateSummary, MultiBaselineAggregateSummary, summarize_multi_baseline_aggregates,
};
pub use ordering::{compare_points_lexicographically, sort_points_lexicographically};
pub use output::{
    BenchmarkRunOverview, render_benchmark_overview, render_multi_baseline_summary,
    render_named_baseline_suite_report, render_suite_report,
};
pub use pruning::{PruningEfficiencyReport, pruning_efficiency_report};
pub use selectivity::{
    SelectivityBucket, SelectivityBucketSummary, SelectivityBucketedWorkloadSummary,
    render_selectivity_bucketed_workload_summary, summarize_workloads_by_selectivity,
};
pub use summary::{
    AggregateWorkloadMetrics, WorkloadComparisonSummary, aggregate_workload_metrics,
    summarize_workload_comparisons,
};
pub use timing::{
    RepeatedComparisonTimingReport, RepeatedTimingConfig, RepeatedTimingReport, TimingReport,
    duration_ratio, measure_elapsed, measure_repeated, measure_repeated_comparison_interleaved,
};
