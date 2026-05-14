use crate::benchmark::{
    BaselineKind, BenchmarkDatasetKind, BenchmarkSuiteConfig, clustered_points_2d,
    clustered_workload_cases, large_clustered_points_2d, large_clustered_workload_cases,
};

#[test]
fn benchmark_suite_config_default_uses_large_dataset() {
    let config = BenchmarkSuiteConfig::default();

    assert_eq!(config.dataset_kind, BenchmarkDatasetKind::LargeClustered2D);
}

#[test]
fn benchmark_suite_config_build_config_matches_values() {
    let config = BenchmarkSuiteConfig::new(
        BenchmarkDatasetKind::SmallClustered2D,
        BaselineKind::FlatScan,
        16,
        12,
        5,
    );

    let build_config = config.build_config();

    assert_eq!(build_config.max_leaf_size, 16);
    assert_eq!(build_config.max_depth, 12);
}

#[test]
fn benchmark_suite_config_timing_config_matches_values() {
    let config = BenchmarkSuiteConfig::new(
        BenchmarkDatasetKind::SmallClustered2D,
        BaselineKind::FlatScan,
        16,
        12,
        5,
    );

    let timing_config = config.timing_config();

    assert_eq!(timing_config.iterations, 5);
}

#[test]
fn benchmark_suite_config_returns_small_dataset() {
    let config = BenchmarkSuiteConfig::new(
        BenchmarkDatasetKind::SmallClustered2D,
        BaselineKind::FlatScan,
        8,
        8,
        10,
    );

    assert_eq!(config.dataset(), clustered_points_2d());
}

#[test]
fn benchmark_suite_config_returns_large_dataset() {
    let config = BenchmarkSuiteConfig::new(
        BenchmarkDatasetKind::LargeClustered2D,
        BaselineKind::FlatScan,
        8,
        8,
        10,
    );

    assert_eq!(config.dataset(), large_clustered_points_2d());
}

#[test]
fn benchmark_suite_config_returns_small_workloads() {
    let config = BenchmarkSuiteConfig::new(
        BenchmarkDatasetKind::SmallClustered2D,
        BaselineKind::FlatScan,
        8,
        8,
        10,
    );

    assert_eq!(config.workloads(), clustered_workload_cases());
}

#[test]
fn benchmark_suite_config_returns_large_workloads() {
    let config = BenchmarkSuiteConfig::new(
        BenchmarkDatasetKind::LargeClustered2D,
        BaselineKind::FlatScan,
        8,
        8,
        10,
    );

    assert_eq!(config.workloads(), large_clustered_workload_cases());
}

#[test]
fn benchmark_suite_config_default_uses_flat_scan_baseline() {
    let config = BenchmarkSuiteConfig::default();

    assert_eq!(config.baseline_kind, BaselineKind::FlatScan);
}

#[test]
fn benchmark_suite_config_can_use_kd_tree_baseline() {
    let config = BenchmarkSuiteConfig::new(
        BenchmarkDatasetKind::SmallClustered2D,
        BaselineKind::KdTree,
        8,
        8,
        10,
    );

    assert_eq!(config.baseline_kind, BaselineKind::KdTree);
}
