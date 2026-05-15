use crate::benchmark::{
    BaselineKind, BenchmarkBaselineSet, BenchmarkDatasetKind, BenchmarkSuiteConfig,
    exact_range_baseline_vec, parse_benchmark_cli_config, parse_benchmark_config,
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
fn parse_benchmark_config_parses_r_tree_baseline() {
    let config = parse_benchmark_config(["--baseline", "r_tree"]).unwrap();

    assert_eq!(config.baseline_kind, BaselineKind::RTree);
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
fn parse_benchmark_cli_config_uses_single_default_baseline() {
    let config = parse_benchmark_cli_config(Vec::<String>::new()).unwrap();

    assert_eq!(
        config.baseline_set,
        BenchmarkBaselineSet::Single(BaselineKind::FlatScan)
    );
    assert_eq!(config.baseline_kinds, vec![BaselineKind::FlatScan]);
}

#[test]
fn parse_benchmark_cli_config_selects_single_baseline() {
    let config = parse_benchmark_cli_config(["--baseline", "kd_tree"]).unwrap();

    assert_eq!(
        config.baseline_set,
        BenchmarkBaselineSet::Single(BaselineKind::KdTree)
    );
    assert_eq!(config.baseline_kinds, vec![BaselineKind::KdTree]);
    assert_eq!(config.suite_config.baseline_kind, BaselineKind::KdTree);
}

#[test]
fn parse_benchmark_cli_config_selects_all_baselines() {
    let config = parse_benchmark_cli_config(["--all-baselines"]).unwrap();

    // keep the cli test tied to the same list the runtime uses
    assert_eq!(config.baseline_set, BenchmarkBaselineSet::AllExact);
    assert_eq!(config.baseline_kinds, exact_range_baseline_vec());
}

#[test]
fn parse_benchmark_cli_config_rejects_baseline_with_all_baselines() {
    let result = parse_benchmark_cli_config(["--baseline", "kd_tree", "--all-baselines"]);

    assert!(result.is_err());
}

#[test]
fn parse_benchmark_cli_config_rejects_all_baselines_with_baseline() {
    let result = parse_benchmark_cli_config(["--all-baselines", "--baseline", "kd_tree"]);

    assert!(result.is_err());
}

#[test]
fn parse_benchmark_cli_config_parses_csv_summary_path() {
    let config = parse_benchmark_cli_config(["--csv-summary", "summary.csv"]).unwrap();

    assert_eq!(config.csv_summary_path, Some("summary.csv".to_string()));
}

#[test]
fn parse_benchmark_cli_config_parses_csv_alias_path() {
    let config = parse_benchmark_cli_config(["--csv", "summary.csv"]).unwrap();

    assert_eq!(config.csv_summary_path, Some("summary.csv".to_string()));
}

#[test]
fn parse_benchmark_cli_config_rejects_missing_csv_summary_value() {
    let result = parse_benchmark_cli_config(["--csv-summary"]);

    assert!(result.is_err());
}

#[test]
fn parse_benchmark_cli_config_parses_csv_workloads_path() {
    let config = parse_benchmark_cli_config(["--csv-workloads", "workloads.csv"]).unwrap();

    assert_eq!(config.csv_workloads_path, Some("workloads.csv".to_string()));
}

#[test]
fn parse_benchmark_cli_config_rejects_missing_csv_workloads_value() {
    let result = parse_benchmark_cli_config(["--csv-workloads"]);

    assert!(result.is_err());
}
