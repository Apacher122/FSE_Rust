use crate::benchmark::{
    BaselineKind, BenchmarkApplicationContext, BenchmarkApplicationError, BenchmarkBaselineSet,
    BenchmarkCliConfig, BenchmarkCsvOutputConfig, BenchmarkDatasetKind, BenchmarkSuiteConfig,
    BenchmarkTerminalOutputMode, run_benchmark_application,
};

#[test]
fn benchmark_application_try_context_accepts_valid_builder_output() {
    let context = BenchmarkApplicationContext::try_from_cli_config(valid_cli_config())
        .expect("valid benchmark config should build a checked context");

    assert!(context.validation.is_valid());
}

#[test]
fn benchmark_application_rejects_invalid_checked_build() {
    let error = run_benchmark_application(invalid_build_cli_config())
        .expect_err("benchmark application should reject invalid index builds");

    assert_eq!(
        error.to_string(),
        "constructed FSE index failed validation: leaf cardinality"
    );

    let BenchmarkApplicationError::BuildValidation(error) = error else {
        panic!("expected build validation error");
    };

    assert!(!error.validation.is_valid());
    assert!(!error.validation.leaf_cardinality_valid);
    assert_eq!(error.diagnostics.leaf_cardinality_violations.len(), 1);

    let violation = &error.diagnostics.leaf_cardinality_violations[0];

    assert_eq!(violation.node_id, 0);
    assert!(violation.cardinality > violation.max_leaf_size);
}

fn valid_cli_config() -> BenchmarkCliConfig {
    benchmark_cli_config(valid_suite_config())
}

fn invalid_build_cli_config() -> BenchmarkCliConfig {
    benchmark_cli_config(invalid_build_suite_config())
}

fn benchmark_cli_config(suite_config: BenchmarkSuiteConfig) -> BenchmarkCliConfig {
    let baseline_set = BenchmarkBaselineSet::Single(BaselineKind::FlatScan);

    BenchmarkCliConfig {
        suite_config,
        baseline_set,
        baseline_kinds: baseline_set.selected_kinds(),
        csv_output: BenchmarkCsvOutputConfig::default(),
        typed_query_index_archive_path: None,
        terminal_output_mode: BenchmarkTerminalOutputMode::Summary,
    }
}

fn valid_suite_config() -> BenchmarkSuiteConfig {
    BenchmarkSuiteConfig::new(
        BenchmarkDatasetKind::SmallClustered2D,
        BaselineKind::FlatScan,
        8,
        8,
        1,
    )
}

fn invalid_build_suite_config() -> BenchmarkSuiteConfig {
    BenchmarkSuiteConfig::new(
        BenchmarkDatasetKind::SmallClustered2D,
        BaselineKind::FlatScan,
        1,
        0,
        1,
    )
}
