use crate::benchmark::{
    BaselineKind, BenchmarkBaselineSet, BenchmarkDatasetKind, BenchmarkSuiteConfig,
    BenchmarkTerminalOutputMode, exact_range_baseline_vec, parse_benchmark_cli_config,
    parse_benchmark_config,
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
fn parse_benchmark_config_allows_repeated_baseline_flags_with_last_value() {
    let config =
        parse_benchmark_config(["--baseline", "flat_scan", "--baseline", "r_tree"]).unwrap();

    // last one wins this was the old parser behavior
    assert_eq!(config.baseline_kind, BaselineKind::RTree);
}

#[test]
fn parse_benchmark_config_parses_small_dataset() {
    let config = parse_benchmark_config(["--dataset", "small"]).unwrap();

    assert_eq!(config.dataset_kind, BenchmarkDatasetKind::SmallClustered2D);
}

#[test]
fn parse_benchmark_config_uses_gap_aware_small_dataset_default_leaf_policy() {
    let config = parse_benchmark_config(["--dataset", "small"]).unwrap();

    assert_eq!(config.dataset_kind, BenchmarkDatasetKind::SmallClustered2D);
    assert_eq!(config.target_leaf_size, 8);
    assert_eq!(config.max_leaf_size, 8);
}

#[test]
fn parse_benchmark_config_keeps_explicit_small_dataset_target_leaf_size() {
    let config = parse_benchmark_config(["--dataset", "small", "--target-leaf-size", "4"]).unwrap();

    assert_eq!(config.dataset_kind, BenchmarkDatasetKind::SmallClustered2D);
    assert_eq!(config.target_leaf_size, 4);
    assert_eq!(config.max_leaf_size, 8);
}

#[test]
fn parse_benchmark_config_keeps_explicit_small_dataset_max_leaf_size() {
    let config = parse_benchmark_config(["--dataset", "small", "--max-leaf-size", "16"]).unwrap();

    assert_eq!(config.dataset_kind, BenchmarkDatasetKind::SmallClustered2D);
    assert_eq!(config.target_leaf_size, 16);
    assert_eq!(config.max_leaf_size, 16);
}

#[test]
fn parse_benchmark_cli_config_uses_gap_aware_small_dataset_default_leaf_policy() {
    let config = parse_benchmark_cli_config(["--dataset", "small"]).unwrap();

    assert_eq!(
        config.suite_config.dataset_kind,
        BenchmarkDatasetKind::SmallClustered2D
    );
    assert_eq!(config.suite_config.target_leaf_size, 8);
    assert_eq!(config.suite_config.max_leaf_size, 8);
}

#[test]
fn parse_benchmark_cli_config_keeps_explicit_small_dataset_leaf_policy() {
    let config =
        parse_benchmark_cli_config(["--dataset", "small", "--target-leaf-size", "4"]).unwrap();

    assert_eq!(
        config.suite_config.dataset_kind,
        BenchmarkDatasetKind::SmallClustered2D
    );
    assert_eq!(config.suite_config.target_leaf_size, 4);
    assert_eq!(config.suite_config.max_leaf_size, 8);
}

#[test]
fn parse_benchmark_config_parses_large_dataset() {
    let config = parse_benchmark_config(["--dataset", "large"]).unwrap();

    assert_eq!(config.dataset_kind, BenchmarkDatasetKind::LargeClustered2D);
}

#[test]
fn parse_benchmark_config_allows_repeated_dataset_flags_with_last_value() {
    let config = parse_benchmark_config(["--dataset", "small", "--dataset", "large"]).unwrap();

    // same rule as the other simple value flags
    assert_eq!(config.dataset_kind, BenchmarkDatasetKind::LargeClustered2D);
}

#[test]
fn parse_benchmark_config_parses_iterations() {
    let config = parse_benchmark_config(["--iterations", "25"]).unwrap();

    assert_eq!(config.timing_iterations, 25);
}

#[test]
fn parse_benchmark_config_allows_repeated_iterations_with_last_value() {
    let config = parse_benchmark_config(["--iterations", "5", "--iterations", "25"]).unwrap();

    assert_eq!(config.timing_iterations, 25);
}

#[test]
fn parse_benchmark_config_parses_leaf_size_and_depth() {
    let config = parse_benchmark_config(["--max-leaf-size", "16", "--max-depth", "12"]).unwrap();

    assert_eq!(config.target_leaf_size, 16);
    assert_eq!(config.max_leaf_size, 16);
    assert_eq!(config.max_depth, 12);
}

#[test]
fn parse_benchmark_config_parses_target_leaf_size() {
    let config = parse_benchmark_config(["--target-leaf-size", "4"]).unwrap();

    assert_eq!(config.target_leaf_size, 4);
    assert_eq!(config.max_leaf_size, 8);
}

#[test]
fn parse_benchmark_config_parses_target_leaf_size_alias() {
    let config = parse_benchmark_config(["--leaf-target-size", "4"]).unwrap();

    assert_eq!(config.target_leaf_size, 4);
    assert_eq!(config.max_leaf_size, 8);
}

#[test]
fn parse_benchmark_config_accepts_target_leaf_size_before_larger_max_leaf_size() {
    let config =
        parse_benchmark_config(["--target-leaf-size", "16", "--max-leaf-size", "32"]).unwrap();

    assert_eq!(config.target_leaf_size, 16);
    assert_eq!(config.max_leaf_size, 32);
}

#[test]
fn parse_benchmark_config_keeps_explicit_target_leaf_size_when_max_leaf_size_changes_later() {
    let config =
        parse_benchmark_config(["--target-leaf-size", "4", "--max-leaf-size", "16"]).unwrap();

    assert_eq!(config.target_leaf_size, 4);
    assert_eq!(config.max_leaf_size, 16);
}

#[test]
fn parse_benchmark_config_updates_target_leaf_size_with_max_leaf_size_when_target_is_not_explicit()
{
    let config = parse_benchmark_config(["--max-leaf-size", "16"]).unwrap();

    // old behavior stays intact unless the new knob is used
    assert_eq!(config.target_leaf_size, 16);
    assert_eq!(config.max_leaf_size, 16);
}

#[test]
fn parse_benchmark_config_allows_repeated_target_leaf_size_with_last_value() {
    let config =
        parse_benchmark_config(["--target-leaf-size", "2", "--target-leaf-size", "4"]).unwrap();

    // last target wins same as other simple flags
    assert_eq!(config.target_leaf_size, 4);
}

#[test]
fn parse_benchmark_config_rejects_target_leaf_size_above_max_leaf_size() {
    let result = parse_benchmark_config(["--target-leaf-size", "16"]);

    assert!(result.is_err());
}

#[test]
fn parse_benchmark_config_rejects_zero_target_leaf_size() {
    let result = parse_benchmark_config(["--target-leaf-size", "0"]);

    assert!(result.is_err());
}

#[test]
fn parse_benchmark_config_rejects_missing_target_leaf_size_value() {
    let result = parse_benchmark_config(["--target-leaf-size"]);

    assert!(result.is_err());
}

#[test]
fn parse_benchmark_config_rejects_non_numeric_target_leaf_size() {
    let result = parse_benchmark_config(["--target-leaf-size", "small"]);

    assert!(result.is_err());
}

#[test]
fn parse_benchmark_config_allows_repeated_leaf_size_and_depth_with_last_value() {
    let config = parse_benchmark_config([
        "--max-leaf-size",
        "8",
        "--max-leaf-size",
        "16",
        "--max-depth",
        "4",
        "--max-depth",
        "12",
    ])
    .unwrap();

    assert_eq!(config.target_leaf_size, 16);
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
fn parse_benchmark_cli_config_uses_empty_csv_output_by_default() {
    let config = parse_benchmark_cli_config(Vec::<String>::new()).unwrap();

    assert!(config.csv_output.is_empty());
    assert!(!config.csv_output.has_summary_output());
    assert!(!config.csv_output.has_workload_output());
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
fn parse_benchmark_cli_config_allows_repeated_single_baselines_with_last_value() {
    let config =
        parse_benchmark_cli_config(["--baseline", "flat_scan", "--baseline", "kd_tree"]).unwrap();

    // this protects the parser state refactor from getting too strict
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
fn parse_benchmark_cli_config_rejects_repeated_baseline_then_all_baselines() {
    let result = parse_benchmark_cli_config([
        "--baseline",
        "flat_scan",
        "--baseline",
        "r_tree",
        "--all-baselines",
    ]);

    assert!(result.is_err());
}

#[test]
fn parse_benchmark_cli_config_rejects_all_baselines_then_repeated_baseline() {
    let result = parse_benchmark_cli_config([
        "--all-baselines",
        "--baseline",
        "flat_scan",
        "--baseline",
        "r_tree",
    ]);

    assert!(result.is_err());
}

#[test]
fn parse_benchmark_cli_config_parses_csv_summary_path() {
    let config = parse_benchmark_cli_config(["--csv-summary", "summary.csv"]).unwrap();

    assert_eq!(
        config.csv_output.summary_path,
        Some("summary.csv".to_string())
    );
    assert!(config.csv_output.has_summary_output());
}

#[test]
fn parse_benchmark_cli_config_parses_csv_alias_path() {
    let config = parse_benchmark_cli_config(["--csv", "summary.csv"]).unwrap();

    assert_eq!(
        config.csv_output.summary_path,
        Some("summary.csv".to_string())
    );
}

#[test]
fn parse_benchmark_cli_config_uses_last_csv_summary_path() {
    let config =
        parse_benchmark_cli_config(["--csv-summary", "first.csv", "--csv-summary", "second.csv"])
            .unwrap();

    assert_eq!(
        config.csv_output.summary_path,
        Some("second.csv".to_string())
    );
}

#[test]
fn parse_benchmark_cli_config_uses_last_csv_alias_or_summary_path() {
    let config =
        parse_benchmark_cli_config(["--csv", "first.csv", "--csv-summary", "second.csv"]).unwrap();

    // csv and csv-summary share the same slot
    assert_eq!(
        config.csv_output.summary_path,
        Some("second.csv".to_string())
    );
}

#[test]
fn parse_benchmark_cli_config_uses_last_csv_summary_or_alias_path() {
    let config =
        parse_benchmark_cli_config(["--csv-summary", "first.csv", "--csv", "second.csv"]).unwrap();

    // same as above but flipped so the alias does not get weird later
    assert_eq!(
        config.csv_output.summary_path,
        Some("second.csv".to_string())
    );
}

#[test]
fn parse_benchmark_cli_config_rejects_missing_csv_summary_value() {
    let result = parse_benchmark_cli_config(["--csv-summary"]);

    assert!(result.is_err());
}

#[test]
fn parse_benchmark_cli_config_parses_csv_workloads_path() {
    let config = parse_benchmark_cli_config(["--csv-workloads", "workloads.csv"]).unwrap();

    assert_eq!(
        config.csv_output.workloads_path,
        Some("workloads.csv".to_string())
    );
    assert!(config.csv_output.has_workload_output());
}

#[test]
fn parse_benchmark_cli_config_uses_last_csv_workloads_path() {
    let config = parse_benchmark_cli_config([
        "--csv-workloads",
        "first.csv",
        "--csv-workloads",
        "second.csv",
    ])
    .unwrap();

    // last one wins here too no special merge behvior
    assert_eq!(
        config.csv_output.workloads_path,
        Some("second.csv".to_string())
    );
}

#[test]
fn parse_benchmark_cli_config_rejects_missing_csv_workloads_value() {
    let result = parse_benchmark_cli_config(["--csv-workloads"]);

    assert!(result.is_err());
}

#[test]
fn parse_benchmark_cli_config_uses_summary_output_by_default() {
    let config = parse_benchmark_cli_config(Vec::<String>::new()).unwrap();

    assert_eq!(
        config.terminal_output_mode,
        BenchmarkTerminalOutputMode::Summary
    );
}

#[test]
fn parse_benchmark_cli_config_parses_debug_report_output_mode() {
    let config = parse_benchmark_cli_config(["--debug-report"]).unwrap();

    assert_eq!(
        config.terminal_output_mode,
        BenchmarkTerminalOutputMode::DebugReport
    );
    assert!(config.terminal_output_mode.is_debug_report());
}
