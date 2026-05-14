use crate::benchmark::{
    BaselineKind, BenchmarkDatasetKind, BenchmarkSuiteConfig, parse_benchmark_config,
};

#[test]
fn parse_benchmark_config_uses_defaults_for_empty_args() {
    let config = parse_benchmark_config(Vec::<String>::new()).unwrap();

    assert_eq!(config, BenchmarkSuiteConfig::default());
}

#[test]
fn parse_benchmark_config_parses_flat_scan_baseline() {
    let config = parse_benchmark_config(["--baseline", "flat_scan"]).unwrap();

    assert_eq!(config.baseline_kind, BaselineKind::FlatScan);
}

#[test]
fn parse_benchmark_config_parses_kd_tree_baseline() {
    let config = parse_benchmark_config(["--baseline", "kd_tree"]).unwrap();

    assert_eq!(config.baseline_kind, BaselineKind::KdTree);
}

#[test]
fn parse_benchmark_config_parses_small_dataset() {
    let config = parse_benchmark_config(["--dataset", "small"]).unwrap();

    assert_eq!(config.dataset_kind, BenchmarkDatasetKind::SmallClustered2D);
}

#[test]
fn parse_benchmark_config_parses_large_dataset() {
    let config = parse_benchmark_config(["--dataset", "large"]).unwrap();

    assert_eq!(config.dataset_kind, BenchmarkDatasetKind::LargeClustered2D);
}

#[test]
fn parse_benchmark_config_parses_iterations() {
    let config = parse_benchmark_config(["--iterations", "25"]).unwrap();

    assert_eq!(config.timing_iterations, 25);
}

#[test]
fn parse_benchmark_config_parses_leaf_size_and_depth() {
    let config = parse_benchmark_config(["--max-leaf-size", "16", "--max-depth", "12"]).unwrap();

    assert_eq!(config.max_leaf_size, 16);
    assert_eq!(config.max_depth, 12);
}

#[test]
fn parse_benchmark_config_rejects_unknown_baseline() {
    let result = parse_benchmark_config(["--baseline", "unknown_tree"]);

    assert!(result.is_err());
}

#[test]
fn parse_benchmark_config_rejects_unknown_argument() {
    let result = parse_benchmark_config(["--unknown"]);

    assert!(result.is_err());
}

#[test]
fn parse_benchmark_config_rejects_missing_value() {
    let result = parse_benchmark_config(["--baseline"]);

    assert!(result.is_err());
}

#[test]
fn parse_benchmark_config_rejects_zero_iterations() {
    let result = parse_benchmark_config(["--iterations", "0"]);

    assert!(result.is_err());
}

#[test]
fn parse_benchmark_config_parses_r_tree_baseline() {
    let config = parse_benchmark_config(["--baseline", "r_tree"]).unwrap();

    assert_eq!(config.baseline_kind, BaselineKind::RTree);
}
