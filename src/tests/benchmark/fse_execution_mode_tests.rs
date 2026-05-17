use crate::benchmark::{
    BaselineKind, BenchmarkApplicationContext, BenchmarkBaselineSet, BenchmarkCliConfig,
    BenchmarkCsvOutputConfig, BenchmarkDatasetKind, BenchmarkSuiteConfig,
    BenchmarkTerminalOutputMode, compare_query_execution_with_options, parse_benchmark_cli_config,
    parse_benchmark_config, run_multi_baseline_benchmark_suite,
    run_multi_baseline_benchmark_suite_with_options,
};
use crate::query::execution::DEFAULT_PARALLEL_MIN_RETAINED_LEAVES;
use crate::query::{QueryExecutionMode, QueryExecutionOptions};
use crate::tests::support::small_benchmark_fixture;

#[test]
fn benchmark_suite_config_defaults_to_serial_fse_execution() {
    let config = BenchmarkSuiteConfig::default();

    assert_eq!(config.fse_execution_mode, QueryExecutionMode::Serial);
    assert_eq!(
        config.fse_parallel_min_retained_leaves,
        DEFAULT_PARALLEL_MIN_RETAINED_LEAVES
    );
    assert_eq!(
        config.query_execution_options(),
        QueryExecutionOptions::serial()
    );
}

#[test]
fn benchmark_suite_config_can_select_parallel_fse_execution() {
    let config =
        BenchmarkSuiteConfig::default().with_fse_execution_mode(QueryExecutionMode::Parallel);

    assert_eq!(config.fse_execution_mode, QueryExecutionMode::Parallel);
    assert_eq!(
        config.query_execution_options(),
        QueryExecutionOptions::parallel()
    );
}

#[test]
fn benchmark_suite_config_can_select_parallel_fse_threshold() {
    let config = BenchmarkSuiteConfig::default()
        .with_fse_execution_mode(QueryExecutionMode::Parallel)
        .with_fse_parallel_min_retained_leaves(2);

    assert_eq!(config.fse_execution_mode, QueryExecutionMode::Parallel);
    assert_eq!(config.fse_parallel_min_retained_leaves, 2);
    assert_eq!(
        config.query_execution_options(),
        QueryExecutionOptions::parallel().with_parallel_min_retained_leaves(2)
    );
}

#[test]
fn benchmark_suite_config_serial_mode_preserves_parallel_threshold() {
    let config = BenchmarkSuiteConfig::default().with_fse_parallel_min_retained_leaves(9);

    assert_eq!(config.fse_execution_mode, QueryExecutionMode::Serial);
    assert_eq!(config.fse_parallel_min_retained_leaves, 9);
    assert_eq!(
        config.query_execution_options(),
        QueryExecutionOptions::serial().with_parallel_min_retained_leaves(9)
    );
}

#[test]
fn parse_benchmark_config_parses_serial_fse_execution_mode() {
    let config = parse_benchmark_config(["--fse-execution", "serial"]).unwrap();

    assert_eq!(config.fse_execution_mode, QueryExecutionMode::Serial);
}

#[test]
fn parse_benchmark_config_parses_parallel_fse_execution_mode() {
    let config = parse_benchmark_config(["--fse-execution", "parallel"]).unwrap();

    assert_eq!(config.fse_execution_mode, QueryExecutionMode::Parallel);
}

#[test]
fn parse_benchmark_config_parses_parallel_fse_threshold() {
    let config = parse_benchmark_config(["--fse-parallel-min-leaves", "2"]).unwrap();

    assert_eq!(config.fse_parallel_min_retained_leaves, 2);
    assert_eq!(
        config.query_execution_options(),
        QueryExecutionOptions::serial().with_parallel_min_retained_leaves(2)
    );
}

#[test]
fn parse_benchmark_config_parses_parallel_fse_threshold_aliases() {
    let config = parse_benchmark_config(["--fse-parallel-min-retained-leaves", "3"]).unwrap();

    assert_eq!(config.fse_parallel_min_retained_leaves, 3);

    let config = parse_benchmark_config(["--fse-parallel-threshold", "5"]).unwrap();

    assert_eq!(config.fse_parallel_min_retained_leaves, 5);
}

#[test]
fn parse_benchmark_config_parses_zero_parallel_fse_threshold() {
    let config = parse_benchmark_config(["--fse-parallel-min-leaves", "0"]).unwrap();

    // zero is allowed so benchmark runs can force rayon
    assert_eq!(config.fse_parallel_min_retained_leaves, 0);
    assert_eq!(
        config.query_execution_options(),
        QueryExecutionOptions::serial().with_parallel_min_retained_leaves(0)
    );
}

#[test]
fn parse_benchmark_config_combines_parallel_mode_and_parallel_threshold() {
    let config = parse_benchmark_config([
        "--fse-execution",
        "parallel",
        "--fse-parallel-min-leaves",
        "1",
    ])
    .unwrap();

    assert_eq!(config.fse_execution_mode, QueryExecutionMode::Parallel);
    assert_eq!(config.fse_parallel_min_retained_leaves, 1);
    assert_eq!(
        config.query_execution_options(),
        QueryExecutionOptions::parallel().with_parallel_min_retained_leaves(1)
    );
}

#[test]
fn parse_benchmark_config_allows_repeated_fse_execution_mode_with_last_value() {
    let config =
        parse_benchmark_config(["--fse-execution", "parallel", "--fse-execution", "serial"])
            .unwrap();

    // last one wins just like the other simple value flags
    assert_eq!(config.fse_execution_mode, QueryExecutionMode::Serial);
}

#[test]
fn parse_benchmark_config_allows_repeated_parallel_threshold_with_last_value() {
    let config = parse_benchmark_config([
        "--fse-parallel-min-leaves",
        "2",
        "--fse-parallel-min-leaves",
        "7",
    ])
    .unwrap();

    // same last value rule as the other simple flags
    assert_eq!(config.fse_parallel_min_retained_leaves, 7);
}

#[test]
fn parse_benchmark_config_parses_fse_mode_alias() {
    let config = parse_benchmark_config(["--fse-mode", "parallel"]).unwrap();

    assert_eq!(config.fse_execution_mode, QueryExecutionMode::Parallel);
}

#[test]
fn parse_benchmark_config_rejects_unknown_fse_execution_mode() {
    let result = parse_benchmark_config(["--fse-execution", "gpu"]);

    assert!(result.is_err());
}

#[test]
fn parse_benchmark_config_rejects_missing_fse_execution_mode_value() {
    let result = parse_benchmark_config(["--fse-execution"]);

    assert!(result.is_err());
}

#[test]
fn parse_benchmark_config_rejects_missing_parallel_threshold_value() {
    let result = parse_benchmark_config(["--fse-parallel-min-leaves"]);

    assert!(result.is_err());
}

#[test]
fn parse_benchmark_config_rejects_non_numeric_parallel_threshold() {
    let result = parse_benchmark_config(["--fse-parallel-min-leaves", "fast"]);

    assert!(result.is_err());
}

#[test]
fn parse_benchmark_cli_config_preserves_fse_execution_mode_in_suite_config() {
    let config = parse_benchmark_cli_config([
        "--dataset",
        "small",
        "--all-baselines",
        "--fse-execution",
        "parallel",
    ])
    .unwrap();

    assert_eq!(
        config.suite_config.dataset_kind,
        BenchmarkDatasetKind::SmallClustered2D
    );
    assert_eq!(config.baseline_set, BenchmarkBaselineSet::AllExact);
    assert_eq!(
        config.suite_config.fse_execution_mode,
        QueryExecutionMode::Parallel
    );
}

#[test]
fn parse_benchmark_cli_config_preserves_parallel_threshold_in_suite_config() {
    let config = parse_benchmark_cli_config([
        "--dataset",
        "small",
        "--all-baselines",
        "--fse-execution",
        "parallel",
        "--fse-parallel-min-leaves",
        "2",
    ])
    .unwrap();

    assert_eq!(
        config.suite_config.dataset_kind,
        BenchmarkDatasetKind::SmallClustered2D
    );
    assert_eq!(config.baseline_set, BenchmarkBaselineSet::AllExact);
    assert_eq!(
        config.suite_config.fse_execution_mode,
        QueryExecutionMode::Parallel
    );
    assert_eq!(config.suite_config.fse_parallel_min_retained_leaves, 2);
    assert_eq!(
        config.suite_config.query_execution_options(),
        QueryExecutionOptions::parallel().with_parallel_min_retained_leaves(2)
    );
}

#[test]
fn benchmark_application_context_preserves_configured_fse_execution_mode() {
    let cli_config = BenchmarkCliConfig {
        suite_config: small_parallel_suite_config(),
        baseline_set: BenchmarkBaselineSet::Single(BaselineKind::FlatScan),
        baseline_kinds: vec![BaselineKind::FlatScan],
        csv_output: BenchmarkCsvOutputConfig::default(),
        terminal_output_mode: BenchmarkTerminalOutputMode::Summary,
    };

    let context = BenchmarkApplicationContext::from_cli_config(cli_config);

    assert_eq!(
        context.suite_config.fse_execution_mode,
        QueryExecutionMode::Parallel
    );
    assert_eq!(
        context.suite_config.query_execution_options(),
        QueryExecutionOptions::parallel().with_parallel_min_retained_leaves(2)
    );
}

#[test]
fn benchmark_comparison_parallel_fse_options_match_serial_fse_options() {
    let fixture = small_benchmark_fixture();
    let workload = &fixture.workloads[0];

    let serial_report = compare_query_execution_with_options(
        &fixture.index,
        &fixture.points,
        &workload.query,
        QueryExecutionOptions::serial(),
    );

    let parallel_report = compare_query_execution_with_options(
        &fixture.index,
        &fixture.points,
        &workload.query,
        QueryExecutionOptions::parallel(),
    );

    assert_eq!(parallel_report.fse_stats, serial_report.fse_stats);
    assert_eq!(parallel_report.baseline_stats, serial_report.baseline_stats);
    assert_eq!(
        parallel_report.candidate_ratio,
        serial_report.candidate_ratio
    );
    assert_eq!(
        parallel_report.retained_leaf_ratio,
        serial_report.retained_leaf_ratio
    );
}

#[test]
fn benchmark_comparison_parallel_threshold_options_match_default_parallel_options() {
    let fixture = small_benchmark_fixture();
    let workload = &fixture.workloads[0];

    let default_parallel_report = compare_query_execution_with_options(
        &fixture.index,
        &fixture.points,
        &workload.query,
        QueryExecutionOptions::parallel(),
    );

    let threshold_parallel_report = compare_query_execution_with_options(
        &fixture.index,
        &fixture.points,
        &workload.query,
        QueryExecutionOptions::parallel().with_parallel_min_retained_leaves(1),
    );

    assert_eq!(
        threshold_parallel_report.fse_stats,
        default_parallel_report.fse_stats
    );
    assert_eq!(
        threshold_parallel_report.baseline_stats,
        default_parallel_report.baseline_stats
    );
}

#[test]
fn multi_baseline_runner_parallel_fse_options_preserve_report_shape() {
    let fixture = small_benchmark_fixture();
    let baseline_kinds = [BaselineKind::FlatScan, BaselineKind::KdTree];

    let serial_report = run_multi_baseline_benchmark_suite(
        &fixture.index,
        &fixture.points,
        &fixture.workloads,
        &fixture.timing_config,
        &fixture.registry,
        &baseline_kinds,
    );

    let parallel_report = run_multi_baseline_benchmark_suite_with_options(
        &fixture.index,
        &fixture.points,
        &fixture.workloads,
        &fixture.timing_config,
        &fixture.registry,
        &baseline_kinds,
        QueryExecutionOptions::parallel().with_parallel_min_retained_leaves(1),
    );

    assert_eq!(
        parallel_report.baseline_reports.len(),
        serial_report.baseline_reports.len()
    );

    for (parallel_baseline, serial_baseline) in parallel_report
        .baseline_reports
        .iter()
        .zip(&serial_report.baseline_reports)
    {
        assert_eq!(
            parallel_baseline.baseline_kind,
            serial_baseline.baseline_kind
        );
        assert_eq!(
            parallel_baseline.baseline_name,
            serial_baseline.baseline_name
        );
        assert_eq!(
            parallel_baseline.report.comparisons.len(),
            serial_baseline.report.comparisons.len()
        );

        for (parallel_comparison, serial_comparison) in parallel_baseline
            .report
            .comparisons
            .iter()
            .zip(&serial_baseline.report.comparisons)
        {
            assert_eq!(
                parallel_comparison.comparison.fse_stats,
                serial_comparison.comparison.fse_stats
            );
            assert_eq!(
                parallel_comparison.comparison.baseline_stats,
                serial_comparison.comparison.baseline_stats
            );
        }
    }
}

fn small_parallel_suite_config() -> BenchmarkSuiteConfig {
    BenchmarkSuiteConfig::new(
        BenchmarkDatasetKind::SmallClustered2D,
        BaselineKind::FlatScan,
        8,
        8,
        3,
    )
    .with_fse_execution_mode(QueryExecutionMode::Parallel)
    .with_fse_parallel_min_retained_leaves(2)
}
