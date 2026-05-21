use crate::benchmark::{
    BenchmarkApplicationContext, BenchmarkApplicationRenderer, BenchmarkApplicationResultBundle,
    BenchmarkBaselineSet, BenchmarkCliConfig, BenchmarkCsvOutputConfig, BenchmarkDatasetKind,
    BenchmarkSuiteConfig, BenchmarkTerminalOutputMode,
};

#[test]
fn debug_report_renders_sibling_overlap_metrics() {
    let context = BenchmarkApplicationContext::from_cli_config(debug_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(output.contains("Sibling overlap metrics"));
    assert!(output.contains("sibling pairs:"));
    assert!(output.contains("overlapping sibling pairs:"));
    assert!(output.contains("total overlap extent:"));
    assert!(output.contains("average overlap extent:"));
    assert!(output.contains("has overlap:"));
}

#[test]
fn debug_report_renders_traversal_pressure_summary() {
    let context = BenchmarkApplicationContext::from_cli_config(debug_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(output.contains("Traversal pressure summary"));
    assert!(output.contains("total visited nodes:"));
    assert!(output.contains("total retained leaves:"));
    assert!(output.contains("total reconstructed records:"));
    assert!(output.contains("visited nodes per retained leaf:"));
    assert!(output.contains("visited nodes per reconstructed record:"));
}

#[test]
fn debug_report_renders_weakest_low_selectivity_workload() {
    let context = BenchmarkApplicationContext::from_cli_config(debug_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(output.contains("Weakest low-selectivity workload"));
    assert!(output.contains(
        "baseline | workload | mean timing | baseline avg | fse avg | visited nodes | fse records | baseline records"
    ));
    assert!(output.contains("kd_tree |"));
    assert!(output.contains("r_tree |"));
    assert!(output.contains("cluster_boundary_range"));
}

#[test]
fn debug_report_renders_boundary_workload_pressure_notes() {
    let context = BenchmarkApplicationContext::from_cli_config(debug_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(output.contains("Boundary workload pressure notes"));
    assert!(output.contains(
        "baseline | workload | timing | baseline records | fse visited | fse retained | fse records | matched | candidate | nodes/record"
    ));
    assert!(output.contains("kd_tree |"));
    assert!(output.contains("r_tree |"));
    assert!(output.contains("cluster_boundary_range"));
}

#[test]
fn debug_report_renders_target_workload_retained_leaf_details() {
    let context = BenchmarkApplicationContext::from_cli_config(debug_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(output.contains("Target workload retained leaf details"));
    assert!(output.contains("workload: cluster_boundary_range"));
    assert!(output.contains("query min:"));
    assert!(output.contains("query max:"));
    assert!(output.contains("retained leaves:"));
    assert!(output.contains(
        "leaf | coverage | records | matched | rejected | overlap volume | leaf volume | overlap ratio | bounds min | bounds max"
    ));
}

#[test]
fn debug_report_renders_target_workload_retained_record_details() {
    let context = BenchmarkApplicationContext::from_cli_config(debug_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(output.contains("Target workload retained record details"));
    assert!(output.contains("leaf | row | result | values"));
    assert!(output.contains("match"));
    assert!(output.contains("reject"));
    assert!(output.contains("[15.00, 15.00]"));
    assert!(output.contains("[54.00, 54.00]"));
}

#[test]
fn compact_report_does_not_render_debug_structure_metrics() {
    let context = BenchmarkApplicationContext::from_cli_config(summary_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(!output.contains("Sibling overlap metrics"));
    assert!(!output.contains("sibling pairs:"));
    assert!(!output.contains("overlapping sibling pairs:"));
    assert!(!output.contains("Traversal pressure summary"));
    assert!(!output.contains("visited nodes per retained leaf:"));
    assert!(!output.contains("Weakest low-selectivity workload"));
    assert!(!output.contains("Boundary workload pressure notes"));
    assert!(!output.contains("Target workload retained leaf details"));
    assert!(!output.contains("Target workload retained record details"));
}

fn summary_cli_config() -> BenchmarkCliConfig {
    BenchmarkCliConfig {
        suite_config: small_fast_suite_config(),
        baseline_set: BenchmarkBaselineSet::AllExact,
        baseline_kinds: BenchmarkBaselineSet::AllExact.selected_kinds(),
        csv_output: BenchmarkCsvOutputConfig::default(),
        terminal_output_mode: BenchmarkTerminalOutputMode::Summary,
    }
}

fn debug_cli_config() -> BenchmarkCliConfig {
    let mut config = summary_cli_config();

    config.terminal_output_mode = BenchmarkTerminalOutputMode::DebugReport;

    config
}

fn small_fast_suite_config() -> BenchmarkSuiteConfig {
    let mut config = BenchmarkSuiteConfig::default();

    config.dataset_kind = BenchmarkDatasetKind::SmallClustered2D;
    config.timing_iterations = 1;
    config.max_leaf_size = 8;
    config.max_depth = 8;

    config
}
