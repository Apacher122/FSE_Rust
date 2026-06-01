use crate::benchmark::{
    BaselineKind, BaselineRegistry, BenchmarkCsvMetadata,
    multi_baseline_low_selectivity_gap_to_csv,
    multi_baseline_low_selectivity_gap_to_csv_with_metadata, parse_benchmark_cli_config,
    run_multi_baseline_benchmark_suite,
};
use crate::tests::support::small_benchmark_fixture;

fn test_metadata() -> BenchmarkCsvMetadata {
    BenchmarkCsvMetadata {
        dataset_records: 60,
        index_nodes: 15,
        workload_count: 6,
        selected_baselines: "flat_scan, kd_tree, r_tree".to_string(),
        timing_iterations: 3,
        target_leaf_size: 8,
        max_leaf_size: 8,
        max_depth: 8,
        fse_execution_mode: "serial".to_string(),
        fse_parallel_min_retained_leaves: 4,
        index_leaf_count: 12,
        index_internal_node_count: 11,
        index_total_leaf_cardinality: 60,
        index_min_leaf_cardinality: 5,
        index_max_leaf_cardinality: 5,
        index_average_leaf_cardinality: 5.0,
        index_total_leaf_volume: 192.0,
        index_average_leaf_volume: 16.0,
        index_density: 0.3125,
        index_zero_volume_leaf_count: 0,
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
fn low_selectivity_gap_csv_includes_tree_baseline_rows() {
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

    let csv = multi_baseline_low_selectivity_gap_to_csv(&report);
    let rows: Vec<&str> = csv.lines().collect();

    assert_eq!(rows.len(), 3);
    assert!(rows[0].starts_with("baseline_name,baseline_label,comparison_label"));
    assert!(rows[0].contains("low_weighted_candidate_ratio"));
    assert!(rows[0].contains("low_mean_timing_ratio"));
    assert!(rows[1].starts_with("kd_tree,KD-Tree,KD-Tree vs FSE,"));
    assert!(rows[2].starts_with("r_tree,R-Tree,R-Tree vs FSE,"));
    assert!(!csv.contains("flat_scan,Flat Scan"));
}

#[test]
fn low_selectivity_gap_csv_includes_metadata_when_requested() {
    let fixture = small_benchmark_fixture();
    let registry = BaselineRegistry::new();
    let baseline_kinds = [BaselineKind::KdTree, BaselineKind::RTree];

    let report = run_multi_baseline_benchmark_suite(
        &fixture.index,
        &fixture.points,
        &fixture.workloads,
        &fixture.timing_config,
        &registry,
        &baseline_kinds,
    );

    let csv = multi_baseline_low_selectivity_gap_to_csv_with_metadata(&test_metadata(), &report);

    assert!(csv.starts_with("dataset_records,index_nodes,run_workload_count"));
    assert!(csv.contains("target_leaf_size,max_leaf_size,max_depth"));
    assert!(csv.contains("low_workload_count,low_baseline_evaluated_records"));
    assert!(csv.contains("60,15,6,"));
}

#[test]
fn low_selectivity_gap_csv_contains_only_header_without_tree_baselines() {
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

    let csv = multi_baseline_low_selectivity_gap_to_csv(&report);
    let rows: Vec<&str> = csv.lines().collect();

    assert_eq!(rows.len(), 1);
    assert!(rows[0].contains("low_weighted_candidate_ratio"));
}

#[test]
fn parse_benchmark_cli_config_parses_low_selectivity_gap_csv_path() {
    let config =
        parse_benchmark_cli_config(["--csv-low-selectivity-gap", "low-selectivity-gap.csv"])
            .unwrap();

    assert_eq!(
        config.csv_output.low_selectivity_gap_path,
        Some("low-selectivity-gap.csv".to_string())
    );
    assert!(config.csv_output.has_low_selectivity_gap_output());
}

#[test]
fn parse_benchmark_cli_config_uses_last_low_selectivity_gap_csv_path() {
    let config = parse_benchmark_cli_config([
        "--csv-low-selectivity-gap",
        "first.csv",
        "--csv-low-gap",
        "second.csv",
    ])
    .unwrap();

    assert_eq!(
        config.csv_output.low_selectivity_gap_path,
        Some("second.csv".to_string())
    );
}

#[test]
fn parse_benchmark_cli_config_rejects_missing_low_selectivity_gap_csv_path() {
    let result = parse_benchmark_cli_config(["--csv-low-selectivity-gap"]);

    assert!(result.is_err());
}
