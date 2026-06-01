use std::time::Duration;

use crate::benchmark::reports::output::format_duration_ascii;
use crate::benchmark::{
    AggregateWorkloadMetrics, BaselineAggregateSummary, BaselineKind, BenchmarkRunOverview,
    BenchmarkSuiteReport, MultiBaselineAggregateSummary, render_benchmark_overview,
    render_multi_baseline_summary, render_named_baseline_suite_report, render_suite_report,
};
use crate::build::{IndexStructureMetrics, IndexValidationReport};
use crate::query::QueryExecutionMode;

#[test]
fn benchmark_overview_render_includes_run_metadata() {
    let overview = test_overview(QueryExecutionMode::Parallel, 2);

    let output = render_benchmark_overview(&overview);

    assert!(output.contains("FSE benchmark suite"));
    assert!(output.contains("Dataset records: 60"));
    assert!(output.contains("Baselines: flat_scan, kd_tree"));
    assert!(output.contains("Target leaf size: 4"));
    assert!(output.contains("Max leaf size: 8"));
    assert!(output.contains("Leaf nodes: 8"));
    assert!(output.contains("Internal nodes: 7"));
    assert!(output.contains("Max leaf cardinality: 8"));
    assert!(output.contains("Average leaf cardinality: 7.50"));
    assert!(output.contains("Total leaf volume: 120.00"));
    assert!(output.contains("Index density: 0.50"));
    assert!(output.contains("Zero-volume leaves: 1"));
    assert!(output.contains("FSE execution: parallel"));
    assert!(output.contains("FSE parallel min leaves: 2"));
    assert!(output.contains("Index validation: true"));
    assert!(output.contains("node identifier consistency valid: true"));
    assert!(output.contains("leaf reconstruction metadata valid: true"));
    assert!(output.contains("leaf record bounds valid: true"));
    assert!(output.contains("leaf ownership cardinality valid: true"));
}

#[test]
fn benchmark_overview_reports_serial_execution_mode_name() {
    let overview = test_overview(QueryExecutionMode::Serial, 4);

    assert_eq!(overview.fse_execution_mode_name(), "serial");

    let output = render_benchmark_overview(&overview);

    assert!(output.contains("FSE execution: serial"));
    assert!(output.contains("FSE parallel min leaves: 4"));
}

#[test]
fn duration_ascii_formatter_uses_portable_units() {
    assert_eq!(format_duration_ascii(Duration::ZERO), "0ns");
    assert_eq!(format_duration_ascii(Duration::from_nanos(999)), "999ns");
    assert_eq!(format_duration_ascii(Duration::from_micros(2)), "2us");
    assert_eq!(
        format_duration_ascii(Duration::from_nanos(2_847)),
        "2.847us"
    );
    assert_eq!(format_duration_ascii(Duration::from_millis(12)), "12ms");
    assert_eq!(
        format_duration_ascii(Duration::from_nanos(12_345_678)),
        "12.345678ms"
    );
    assert_eq!(
        format_duration_ascii(Duration::from_secs(2) + Duration::from_millis(500)),
        "2.5s"
    );
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
fn suite_report_render_uses_ascii_duration_units() {
    let mut aggregate = AggregateWorkloadMetrics::default();
    aggregate.total_baseline_average_elapsed = Duration::from_nanos(2_847);
    aggregate.total_fse_average_elapsed = Duration::from_nanos(1_613);
    aggregate.mean_baseline_average_elapsed = Duration::from_micros(2);
    aggregate.mean_fse_average_elapsed = Duration::from_micros(1);

    let report = BenchmarkSuiteReport {
        comparisons: Vec::new(),
        aggregate,
        pruning_reports: Vec::new(),
    };

    let output = render_suite_report(&report);

    assert!(output.contains("total baseline average elapsed: 2.847us"));
    assert!(output.contains("total FSE average elapsed: 1.613us"));
    assert!(output.contains("mean baseline average elapsed: 2us"));
    assert!(output.contains("mean FSE average elapsed: 1us"));
    assert!(output.is_ascii());
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

#[test]
fn multi_baseline_summary_render_uses_ascii_duration_units() {
    let summary = MultiBaselineAggregateSummary {
        baseline_summaries: vec![BaselineAggregateSummary {
            baseline_kind: BaselineKind::FlatScan,
            baseline_name: "flat_scan".to_string(),
            baseline_label: "Flat Scan".to_string(),
            comparison_label: "Flat Scan vs FSE".to_string(),
            workload_count: 1,
            total_baseline_evaluated_records: 60,
            total_fse_reconstructed_records: 10,
            weighted_reconstruction_avoidance_ratio: 0.83,
            weighted_candidate_ratio: 0.17,
            mean_timing_ratio: 1.25,
            weighted_timing_ratio: 1.25,
            total_baseline_average_elapsed: Duration::from_nanos(3_470),
            total_fse_average_elapsed: Duration::from_nanos(2_847),
        }],
    };

    let output = render_multi_baseline_summary(&summary);

    assert!(output.contains("total baseline average elapsed: 3.47us"));
    assert!(output.contains("total FSE average elapsed: 2.847us"));
    assert!(output.is_ascii());
}

fn test_overview(
    fse_execution_mode: QueryExecutionMode,
    fse_parallel_min_retained_leaves: usize,
) -> BenchmarkRunOverview {
    BenchmarkRunOverview {
        dataset_records: 60,
        index_nodes: 15,
        workloads: 6,
        baselines: "flat_scan, kd_tree".to_string(),
        timing_iterations: 3,
        target_leaf_size: 4,
        max_leaf_size: 8,
        max_depth: 8,
        fse_execution_mode,
        fse_parallel_min_retained_leaves,
        index_structure: IndexStructureMetrics {
            node_count: 15,
            leaf_count: 8,
            internal_node_count: 7,
            total_leaf_cardinality: 60,
            min_leaf_cardinality: 4,
            max_leaf_cardinality: 8,
            average_leaf_cardinality: 7.5,
            total_leaf_volume: 120.0,
            average_leaf_volume: 15.0,
            index_density: 0.5,
            zero_volume_leaf_count: 1,
        },
        validation: IndexValidationReport {
            node_identifier_consistency_valid: true,
            leaf_cardinality_valid: true,
            leaf_reconstruction_metadata_valid: true,
            leaf_record_bounds_valid: true,
            leaf_ownership_cardinality_valid: true,
            hierarchy_topology_valid: true,
            parent_child_bounds_valid: true,
        },
    }
}
