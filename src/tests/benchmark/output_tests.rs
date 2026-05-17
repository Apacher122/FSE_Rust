use std::time::Duration;

use crate::benchmark::{
    AggregateWorkloadMetrics, BaselineAggregateSummary, BaselineKind, BenchmarkRunOverview,
    BenchmarkSuiteReport, MultiBaselineAggregateSummary, render_benchmark_overview,
    render_multi_baseline_summary, render_named_baseline_suite_report, render_suite_report,
};
use crate::build::IndexValidationReport;
use crate::query::QueryExecutionMode;

#[test]
fn benchmark_overview_render_includes_run_metadata() {
    let overview = BenchmarkRunOverview {
        dataset_records: 60,
        index_nodes: 15,
        workloads: 6,
        baselines: "flat_scan, kd_tree".to_string(),
        timing_iterations: 3,
        max_leaf_size: 8,
        max_depth: 8,
        fse_execution_mode: QueryExecutionMode::Parallel,
        fse_parallel_min_retained_leaves: 2,
        validation: IndexValidationReport {
            leaf_cardinality_valid: true,
            hierarchy_topology_valid: true,
            parent_child_bounds_valid: true,
        },
    };

    let output = render_benchmark_overview(&overview);

    assert!(output.contains("FSE benchmark suite"));
    assert!(output.contains("Dataset records: 60"));
    assert!(output.contains("Baselines: flat_scan, kd_tree"));
    assert!(output.contains("FSE execution: parallel"));
    assert!(output.contains("FSE parallel min leaves: 2"));
    assert!(output.contains("Index validation: true"));
}

#[test]
fn benchmark_overview_reports_serial_execution_mode_name() {
    let overview = BenchmarkRunOverview {
        dataset_records: 60,
        index_nodes: 15,
        workloads: 6,
        baselines: "flat_scan".to_string(),
        timing_iterations: 3,
        max_leaf_size: 8,
        max_depth: 8,
        fse_execution_mode: QueryExecutionMode::Serial,
        fse_parallel_min_retained_leaves: 4,
        validation: IndexValidationReport {
            leaf_cardinality_valid: true,
            hierarchy_topology_valid: true,
            parent_child_bounds_valid: true,
        },
    };

    assert_eq!(overview.fse_execution_mode_name(), "serial");

    let output = render_benchmark_overview(&overview);

    assert!(output.contains("FSE execution: serial"));
    assert!(output.contains("FSE parallel min leaves: 4"));
}

#[test]
fn suite_report_render_includes_aggregate_heading() {
    let report = BenchmarkSuiteReport {
        comparisons: Vec::new(),
        aggregate: AggregateWorkloadMetrics::default(),
        pruning_reports: Vec::new(),
    };

    let output = render_suite_report(&report);

    assert!(output.contains("Aggregate workload metrics"));
    assert!(output.contains("total baseline evaluated records: 0"));
}

#[test]
fn named_baseline_suite_report_render_includes_baseline_name() {
    let report = BenchmarkSuiteReport {
        comparisons: Vec::new(),
        aggregate: AggregateWorkloadMetrics::default(),
        pruning_reports: Vec::new(),
    };

    let output = render_named_baseline_suite_report("flat_scan", &report);

    assert!(output.contains("Baseline suite: flat_scan"));
    assert!(output.contains("Aggregate workload metrics"));
}

#[test]
fn multi_baseline_summary_render_includes_baseline_rows() {
    let summary = MultiBaselineAggregateSummary {
        baseline_summaries: vec![BaselineAggregateSummary {
            baseline_kind: BaselineKind::FlatScan,
            baseline_name: "flat_scan".to_string(),
            baseline_label: "Flat Scan".to_string(),
            comparison_label: "Flat Scan vs FSE".to_string(),
            workload_count: 3,
            total_baseline_evaluated_records: 300,
            total_fse_reconstructed_records: 75,
            weighted_reconstruction_avoidance_ratio: 0.75,
            weighted_candidate_ratio: 0.25,
            mean_timing_ratio: 1.5,
            weighted_timing_ratio: 1.25,
            total_baseline_average_elapsed: Duration::from_nanos(100),
            total_fse_average_elapsed: Duration::from_nanos(80),
        }],
    };

    let output = render_multi_baseline_summary(&summary);

    assert!(output.contains("Multi-baseline aggregate summary"));
    assert!(output.contains("Baseline: Flat Scan"));
    assert!(output.contains("Comparison: Flat Scan vs FSE"));
    assert!(output.contains("Highest weighted timing ratio: Flat Scan (1.25)"));
}
