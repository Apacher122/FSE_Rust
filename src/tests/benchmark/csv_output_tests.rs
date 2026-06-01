use std::fs;

use crate::benchmark::{
    BaselineKind, BaselineRegistry, BenchmarkCsvMetadata, BenchmarkCsvOutputConfig,
    run_multi_baseline_benchmark_suite, summarize_multi_baseline_aggregates,
    write_benchmark_csv_outputs,
};
use crate::tests::support::small_benchmark_fixture;

fn test_metadata() -> BenchmarkCsvMetadata {
    BenchmarkCsvMetadata {
        dataset_records: 60,
        index_nodes: 15,
        workload_count: 6,
        selected_baselines: "flat_scan".to_string(),
        timing_iterations: 3,
        target_leaf_size: 4,
        max_leaf_size: 8,
        max_depth: 8,
        fse_execution_mode: "parallel".to_string(),
        fse_parallel_min_retained_leaves: 2,
        index_leaf_count: 8,
        index_internal_node_count: 7,
        index_total_leaf_cardinality: 60,
        index_min_leaf_cardinality: 4,
        index_max_leaf_cardinality: 8,
        index_average_leaf_cardinality: 7.5,
        index_total_leaf_volume: 120.0,
        index_average_leaf_volume: 15.0,
        index_density: 0.5,
        index_zero_volume_leaf_count: 1,
        index_valid: true,
        node_identifier_consistency_valid: true,
        leaf_cardinality_valid: true,
        leaf_record_bounds_valid: true,
        leaf_ownership_cardinality_valid: true,
        hierarchy_topology_valid: true,
        parent_child_bounds_valid: true,
    }
}

#[test]
fn benchmark_csv_output_writer_returns_empty_report_when_no_paths_are_configured() {
    let fixture = small_benchmark_fixture();
    let registry = BaselineRegistry::new();
    let baseline_kinds = [
        BaselineKind::FlatScan,
        BaselineKind::KdTree,
        BaselineKind::RTree,
    ];

    let report = run_multi_baseline_benchmark_suite(
        &fixture.index,
        &fixture.points,
        &fixture.workloads,
        &fixture.timing_config,
        &registry,
        &baseline_kinds,
    );

    let aggregate_summary = summarize_multi_baseline_aggregates(&report);
    let csv_output = BenchmarkCsvOutputConfig::default();

    let write_report =
        write_benchmark_csv_outputs(&csv_output, &test_metadata(), &aggregate_summary, &report)
            .unwrap();

    assert!(write_report.is_empty());
    assert!(write_report.status_lines().is_empty());
}

#[test]
fn benchmark_csv_output_writer_writes_configured_outputs() {
    let fixture = small_benchmark_fixture();
    let registry = BaselineRegistry::new();
    let baseline_kinds = [
        BaselineKind::FlatScan,
        BaselineKind::KdTree,
        BaselineKind::RTree,
    ];

    let report = run_multi_baseline_benchmark_suite(
        &fixture.index,
        &fixture.points,
        &fixture.workloads,
        &fixture.timing_config,
        &registry,
        &baseline_kinds,
    );

    let aggregate_summary = summarize_multi_baseline_aggregates(&report);

    let summary_path = std::env::temp_dir().join(format!(
        "fse_csv_output_summary_test_{}.csv",
        std::process::id()
    ));
    let workloads_path = std::env::temp_dir().join(format!(
        "fse_csv_output_workloads_test_{}.csv",
        std::process::id()
    ));
    let low_selectivity_gap_path = std::env::temp_dir().join(format!(
        "fse_csv_output_low_selectivity_gap_test_{}.csv",
        std::process::id()
    ));

    let summary_path_text = summary_path.to_string_lossy().to_string();
    let workloads_path_text = workloads_path.to_string_lossy().to_string();
    let low_selectivity_gap_path_text = low_selectivity_gap_path.to_string_lossy().to_string();

    let mut csv_output = BenchmarkCsvOutputConfig::new(
        Some(summary_path_text.clone()),
        Some(workloads_path_text.clone()),
    );
    csv_output.set_low_selectivity_gap_path(low_selectivity_gap_path_text.clone());

    let write_report =
        write_benchmark_csv_outputs(&csv_output, &test_metadata(), &aggregate_summary, &report)
            .unwrap();

    assert_eq!(write_report.summary_path, Some(summary_path_text.clone()));
    assert_eq!(
        write_report.workloads_path,
        Some(workloads_path_text.clone())
    );
    assert_eq!(
        write_report.low_selectivity_gap_path,
        Some(low_selectivity_gap_path_text.clone())
    );

    let summary_csv = fs::read_to_string(&summary_path).unwrap();
    let workloads_csv = fs::read_to_string(&workloads_path).unwrap();
    let low_selectivity_gap_csv = fs::read_to_string(&low_selectivity_gap_path).unwrap();

    assert!(summary_csv.contains("baseline_name,baseline_label,comparison_label"));
    assert!(summary_csv.contains("target_leaf_size,max_leaf_size,max_depth"));
    assert!(summary_csv.contains("index_leaf_count,index_internal_node_count"));
    assert!(summary_csv.contains("parallel,2,8,7,60,4,8,7.500000"));
    assert!(workloads_csv.contains("baseline_name,baseline_label,comparison_label,workload_name"));
    assert!(workloads_csv.contains("target_leaf_size,max_leaf_size,max_depth"));
    assert!(workloads_csv.contains("index_leaf_count,index_internal_node_count"));
    assert!(workloads_csv.contains("parallel,2,8,7,60,4,8,7.500000"));
    assert!(low_selectivity_gap_csv.contains("baseline_name,baseline_label,comparison_label"));
    assert!(low_selectivity_gap_csv.contains("low_weighted_candidate_ratio"));
    assert!(low_selectivity_gap_csv.contains("parallel,2,8,7,60,4,8,7.500000"));

    let _ = fs::remove_file(summary_path);
    let _ = fs::remove_file(workloads_path);
    let _ = fs::remove_file(low_selectivity_gap_path);
}

#[test]
fn benchmark_csv_output_writer_reports_status_lines_in_write_order() {
    let write_report = crate::benchmark::BenchmarkCsvWriteReport {
        summary_path: Some("summary.csv".to_string()),
        workloads_path: Some("workloads.csv".to_string()),
        low_selectivity_gap_path: Some("low-gap.csv".to_string()),
    };

    // keep this matching the old main output exactly
    assert_eq!(
        write_report.status_lines(),
        vec![
            "CSV summary written: summary.csv".to_string(),
            "Workload CSV written: workloads.csv".to_string(),
            "Low-selectivity gap CSV written: low-gap.csv".to_string(),
        ]
    );
}
