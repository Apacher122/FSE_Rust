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
        max_leaf_size: 8,
        max_depth: 8,
        index_valid: true,
        leaf_cardinality_valid: true,
        hierarchy_topology_valid: true,
        parent_child_bounds_valid: true,
    }
}

#[test]
fn benchmark_csv_output_writer_returns_empty_report_when_no_paths_are_configured() {
    let fixture = small_benchmark_fixture();
    let registry = BaselineRegistry::new();
    let baseline_kinds = [BaselineKind::FlatScan];

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
    let baseline_kinds = [BaselineKind::FlatScan];

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

    let summary_path_text = summary_path.to_string_lossy().to_string();
    let workloads_path_text = workloads_path.to_string_lossy().to_string();

    let csv_output = BenchmarkCsvOutputConfig::new(
        Some(summary_path_text.clone()),
        Some(workloads_path_text.clone()),
    );

    let write_report =
        write_benchmark_csv_outputs(&csv_output, &test_metadata(), &aggregate_summary, &report)
            .unwrap();

    assert_eq!(write_report.summary_path, Some(summary_path_text.clone()));
    assert_eq!(
        write_report.workloads_path,
        Some(workloads_path_text.clone())
    );

    let summary_csv = fs::read_to_string(&summary_path).unwrap();
    let workloads_csv = fs::read_to_string(&workloads_path).unwrap();

    assert!(summary_csv.contains("baseline_name,baseline_label,comparison_label"));
    assert!(workloads_csv.contains("baseline_name,baseline_label,comparison_label,workload_name"));

    let _ = fs::remove_file(summary_path);
    let _ = fs::remove_file(workloads_path);
}

#[test]
fn benchmark_csv_output_writer_reports_status_lines_in_write_order() {
    let write_report = crate::benchmark::BenchmarkCsvWriteReport {
        summary_path: Some("summary.csv".to_string()),
        workloads_path: Some("workloads.csv".to_string()),
    };

    // keep this matching the old main output exactly
    assert_eq!(
        write_report.status_lines(),
        vec![
            "CSV summary written: summary.csv".to_string(),
            "Workload CSV written: workloads.csv".to_string(),
        ]
    );
}
