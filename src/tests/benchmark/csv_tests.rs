use crate::benchmark::{
    BaselineAggregateSummary, BaselineKind, BaselineRegistry, BenchmarkCsvMetadata,
    MultiBaselineAggregateSummary, multi_baseline_aggregate_summary_to_csv,
    multi_baseline_aggregate_summary_to_csv_with_metadata, multi_baseline_workload_report_to_csv,
    multi_baseline_workload_report_to_csv_with_metadata, run_multi_baseline_benchmark_suite,
    write_multi_baseline_aggregate_summary_csv,
    write_multi_baseline_aggregate_summary_csv_with_metadata,
    write_multi_baseline_workload_report_csv_with_metadata,
};
use crate::tests::support::small_benchmark_fixture;
use std::fs;
use std::time::Duration;

fn test_metadata() -> BenchmarkCsvMetadata {
    BenchmarkCsvMetadata {
        dataset_records: 60,
        index_nodes: 15,
        workload_count: 6,
        selected_baselines: "flat_scan, kd_tree".to_string(),
        timing_iterations: 3,
        max_leaf_size: 8,
        max_depth: 8,
        index_valid: true,
        leaf_cardinality_valid: true,
        hierarchy_topology_valid: true,
        parent_child_bounds_valid: true,
    }
}

fn single_summary() -> MultiBaselineAggregateSummary {
    MultiBaselineAggregateSummary {
        baseline_summaries: vec![BaselineAggregateSummary {
            baseline_kind: BaselineKind::FlatScan,
            baseline_name: "flat_scan".to_string(),
            baseline_label: "Flat Scan".to_string(),
            comparison_label: "Flat Scan vs FSE".to_string(),
            workload_count: 1,
            total_baseline_evaluated_records: 100,
            total_fse_reconstructed_records: 25,
            weighted_reconstruction_avoidance_ratio: 0.75,
            weighted_candidate_ratio: 0.25,
            mean_timing_ratio: 1.5,
            weighted_timing_ratio: 1.25,
            total_baseline_average_elapsed: Duration::from_nanos(100),
            total_fse_average_elapsed: Duration::from_nanos(80),
        }],
    }
}

#[test]
fn csv_export_includes_header() {
    let summary = MultiBaselineAggregateSummary::default();

    let csv = multi_baseline_aggregate_summary_to_csv(&summary);

    assert!(csv.starts_with("baseline_name,baseline_label,comparison_label,workload_count"));
}

#[test]
fn csv_export_includes_one_row_per_baseline_summary() {
    let summary = MultiBaselineAggregateSummary {
        baseline_summaries: vec![
            BaselineAggregateSummary {
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
            },
            BaselineAggregateSummary {
                baseline_kind: BaselineKind::KdTree,
                baseline_name: "kd_tree".to_string(),
                baseline_label: "KD-Tree".to_string(),
                comparison_label: "KD-Tree vs FSE".to_string(),
                workload_count: 3,
                total_baseline_evaluated_records: 120,
                total_fse_reconstructed_records: 75,
                weighted_reconstruction_avoidance_ratio: 0.375,
                weighted_candidate_ratio: 0.625,
                mean_timing_ratio: 0.8,
                weighted_timing_ratio: 0.9,
                total_baseline_average_elapsed: Duration::from_nanos(90),
                total_fse_average_elapsed: Duration::from_nanos(100),
            },
        ],
    };

    let csv = multi_baseline_aggregate_summary_to_csv(&summary);
    let rows: Vec<&str> = csv.lines().collect();

    assert_eq!(rows.len(), 3);
    assert!(rows[1].starts_with("flat_scan,Flat Scan,Flat Scan vs FSE,3"));
    assert!(rows[2].starts_with("kd_tree,KD-Tree,KD-Tree vs FSE,3"));
}

#[test]
fn csv_export_formats_ratios_with_fixed_precision() {
    let summary = single_summary();

    let csv = multi_baseline_aggregate_summary_to_csv(&summary);

    assert!(csv.contains("0.750000"));
    assert!(csv.contains("0.250000"));
    assert!(csv.contains("1.500000"));
    assert!(csv.contains("1.250000"));
}

#[test]
fn csv_export_escapes_fields_that_need_quotes() {
    let summary = MultiBaselineAggregateSummary {
        baseline_summaries: vec![BaselineAggregateSummary {
            baseline_kind: BaselineKind::FlatScan,
            baseline_name: "custom_baseline".to_string(),
            baseline_label: "Custom, Baseline".to_string(),
            comparison_label: "Custom \"Baseline\" vs FSE".to_string(),
            workload_count: 1,
            total_baseline_evaluated_records: 10,
            total_fse_reconstructed_records: 5,
            weighted_reconstruction_avoidance_ratio: 0.5,
            weighted_candidate_ratio: 0.5,
            mean_timing_ratio: 1.0,
            weighted_timing_ratio: 1.0,
            total_baseline_average_elapsed: Duration::from_nanos(10),
            total_fse_average_elapsed: Duration::from_nanos(10),
        }],
    };

    let csv = multi_baseline_aggregate_summary_to_csv(&summary);

    assert!(csv.contains("\"Custom, Baseline\""));
    assert!(csv.contains("\"Custom \"\"Baseline\"\" vs FSE\""));
}

#[test]
fn csv_export_writes_summary_file() {
    let summary = single_summary();

    let path = std::env::temp_dir().join(format!("fse_summary_test_{}.csv", std::process::id()));

    write_multi_baseline_aggregate_summary_csv(&path, &summary).unwrap();

    let written = fs::read_to_string(&path).unwrap();

    assert!(written.contains("flat_scan,Flat Scan,Flat Scan vs FSE,1"));

    let _ = fs::remove_file(path);
}

#[test]
fn csv_export_with_metadata_includes_metadata_header() {
    let metadata = test_metadata();
    let summary = single_summary();

    let csv = multi_baseline_aggregate_summary_to_csv_with_metadata(&metadata, &summary);

    assert!(csv.starts_with("dataset_records,index_nodes,run_workload_count,selected_baselines"));
}

#[test]
fn csv_export_with_metadata_includes_metadata_values() {
    let metadata = test_metadata();
    let summary = single_summary();

    let csv = multi_baseline_aggregate_summary_to_csv_with_metadata(&metadata, &summary);
    let rows: Vec<&str> = csv.lines().collect();

    assert_eq!(rows.len(), 2);
    assert!(rows[1].starts_with("60,15,6,\"flat_scan, kd_tree\",3,8,8,true,true,true,true"));
}

#[test]
fn csv_export_with_metadata_writes_summary_file() {
    let metadata = test_metadata();
    let summary = single_summary();

    let path = std::env::temp_dir().join(format!(
        "fse_summary_metadata_test_{}.csv",
        std::process::id()
    ));

    write_multi_baseline_aggregate_summary_csv_with_metadata(&path, &metadata, &summary).unwrap();

    let written = fs::read_to_string(&path).unwrap();

    assert!(written.contains("dataset_records,index_nodes,run_workload_count"));
    assert!(written.contains("flat_scan,Flat Scan,Flat Scan vs FSE,1"));

    let _ = fs::remove_file(path);
}

#[test]
fn workload_csv_export_includes_header() {
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

    let csv = multi_baseline_workload_report_to_csv(&report);

    assert!(csv.starts_with("baseline_name,baseline_label,comparison_label,workload_name"));
}

#[test]
fn workload_csv_export_includes_one_row_per_workload() {
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

    let csv = multi_baseline_workload_report_to_csv(&report);
    let rows: Vec<&str> = csv.lines().collect();

    assert_eq!(rows.len(), fixture.workloads.len() + 1);
}

#[test]
fn workload_csv_export_with_metadata_includes_metadata_header() {
    let fixture = small_benchmark_fixture();
    let metadata = test_metadata();
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

    let csv = multi_baseline_workload_report_to_csv_with_metadata(&metadata, &report);

    assert!(csv.starts_with("dataset_records,index_nodes,run_workload_count,selected_baselines"));
    assert!(csv.contains("baseline_name,baseline_label,comparison_label,workload_name"));
}

#[test]
fn workload_csv_export_writes_metadata_file() {
    let fixture = small_benchmark_fixture();
    let metadata = test_metadata();
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

    let path = std::env::temp_dir().join(format!(
        "fse_workload_metadata_test_{}.csv",
        std::process::id()
    ));

    write_multi_baseline_workload_report_csv_with_metadata(&path, &metadata, &report).unwrap();

    let written = fs::read_to_string(&path).unwrap();

    assert!(written.contains("workload_name"));
    assert!(written.contains("flat_scan,Flat Scan,Flat Scan vs FSE"));

    let _ = fs::remove_file(path);
}
