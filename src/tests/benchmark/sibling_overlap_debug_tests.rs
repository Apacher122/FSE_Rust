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
    assert!(output.contains("leaf | coverage | records | bounds min | bounds max | volume"));
}

#[test]
fn debug_report_uses_large_dataset_target_workload() {
    let context = BenchmarkApplicationContext::from_cli_config(large_debug_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(output.contains("Target workload retained leaf details"));
    assert!(output.contains("workload: large_cross_cluster_boundary"));
    assert!(output.contains("Target workload resultless timing estimate"));
    assert!(output.contains("Target workload count-only comparison"));
    assert!(output.contains("matched records agree: true"));
    assert!(output.contains("query min:"));
    assert!(output.contains("query max:"));
    assert!(output.contains("retained leaves:"));
    assert!(!output.contains("workload: cluster_boundary_range\nstatus: workload not found"));
}

#[test]
fn debug_report_renders_target_workload_stage_timing_estimate() {
    let context = BenchmarkApplicationContext::from_cli_config(debug_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(output.contains("Target workload stage timing estimate"));
    assert!(output.contains("workload: cluster_boundary_range"));
    assert!(output.contains("timing iterations:"));
    assert!(output.contains("average traversal elapsed:"));
    assert!(output.contains("average full FSE elapsed:"));
    assert!(output.contains("estimated non-traversal elapsed:"));
    assert!(output.contains("estimated traversal share:"));
    assert!(output.contains("retained leaves:"));
    assert!(output.contains("candidate records:"));
    assert!(output.contains("matched records:"));
}

#[test]
fn debug_report_renders_target_workload_retained_execution_phase_estimate() {
    let context = BenchmarkApplicationContext::from_cli_config(debug_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(output.contains("Target workload retained execution phase estimate"));
    assert!(output.contains("workload: cluster_boundary_range"));
    assert!(output.contains("timing iterations:"));
    assert!(output.contains("average retained reconstruction elapsed:"));
    assert!(output.contains("average retained predicate elapsed:"));
    assert!(output.contains("average retained result collection elapsed:"));
    assert!(output.contains("average retained execution elapsed:"));
    assert!(output.contains("candidate records:"));
    assert!(output.contains("matched records:"));
    assert!(output.contains("covered leaves:"));
    assert!(output.contains("partial leaves:"));
    assert!(output.contains("covered records:"));
    assert!(output.contains("partial records:"));
}

#[test]
fn debug_report_renders_target_workload_retained_allocation_estimate() {
    let context = BenchmarkApplicationContext::from_cli_config(debug_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(output.contains("Target workload retained allocation estimate"));
    assert!(output.contains("workload: cluster_boundary_range"));
    assert!(output.contains("timing iterations:"));
    assert!(output.contains("candidate records:"));
    assert!(output.contains("matched records:"));
    assert!(output.contains("covered leaves:"));
    assert!(output.contains("partial leaves:"));
    assert!(output.contains("covered records:"));
    assert!(output.contains("partial records:"));
    assert!(output.contains("average empty result allocation elapsed:"));
    assert!(output.contains("average matched result allocation elapsed:"));
    assert!(output.contains("average candidate result allocation elapsed:"));
    assert!(output.contains("average vector clone collection elapsed:"));
    assert!(output.contains("average retained execution elapsed:"));
}

#[test]
fn debug_report_renders_target_workload_resultless_timing_estimate() {
    let context = BenchmarkApplicationContext::from_cli_config(debug_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(output.contains("Target workload resultless timing estimate"));
    assert!(output.contains("workload: cluster_boundary_range"));
    assert!(output.contains("timing iterations:"));
    assert!(output.contains("average resultless retained elapsed:"));
    assert!(output.contains("average retained execution elapsed:"));
    assert!(output.contains("average full FSE elapsed:"));
    assert!(output.contains("estimated result ownership overhead:"));
    assert!(output.contains("estimated resultless retained share:"));
    assert!(output.contains("candidate records:"));
    assert!(output.contains("matched records:"));
    assert!(output.contains("covered leaves:"));
    assert!(output.contains("partial leaves:"));
    assert!(output.contains("covered records:"));
    assert!(output.contains("partial records:"));
}

#[test]
fn debug_report_renders_target_workload_count_only_comparison() {
    let context = BenchmarkApplicationContext::from_cli_config(debug_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(output.contains("Target workload count-only comparison"));
    assert!(output.contains("workload: cluster_boundary_range"));
    assert!(output.contains("timing iterations:"));
    assert!(output.contains("owned average elapsed:"));
    assert!(output.contains("count-only average elapsed:"));
    assert!(output.contains("estimated owned result overhead:"));
    assert!(output.contains("count-only speedup:"));
    assert!(output.contains("owned matched records:"));
    assert!(output.contains("count-only matched records:"));
    assert!(output.contains("matched records agree: true"));
    assert!(output.contains("owned candidate records:"));
    assert!(output.contains("count-only candidate records:"));
    assert!(output.contains("owned retained leaves:"));
    assert!(output.contains("count-only retained leaves:"));
    assert!(output.contains("owned visited nodes:"));
    assert!(output.contains("count-only visited nodes:"));
}

#[test]
fn debug_report_renders_target_workload_reconstruction_timing_estimate() {
    let context = BenchmarkApplicationContext::from_cli_config(debug_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(output.contains("Target workload reconstruction timing estimate"));
    assert!(output.contains("workload: cluster_boundary_range"));
    assert!(output.contains("timing iterations:"));
    assert!(output.contains("average retained execution elapsed:"));
    assert!(output.contains("average full FSE elapsed:"));
    assert!(output.contains("estimated retained execution share:"));
    assert!(output.contains("retained leaves:"));
    assert!(output.contains("covered leaves:"));
    assert!(output.contains("partial leaves:"));
    assert!(output.contains("candidate records:"));
    assert!(output.contains("covered records:"));
    assert!(output.contains("partial records:"));
    assert!(output.contains("matched records:"));
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
    assert!(!output.contains("Target workload stage timing estimate"));
    assert!(!output.contains("average traversal elapsed:"));
    assert!(!output.contains("Target workload reconstruction timing estimate"));
    assert!(!output.contains("average retained execution elapsed:"));
    assert!(!output.contains("Target workload retained execution phase estimate"));
    assert!(!output.contains("average retained reconstruction elapsed:"));
    assert!(!output.contains("average retained predicate elapsed:"));
    assert!(!output.contains("average retained result collection elapsed:"));
    assert!(!output.contains("Target workload retained allocation estimate"));
    assert!(!output.contains("average empty result allocation elapsed:"));
    assert!(!output.contains("average matched result allocation elapsed:"));
    assert!(!output.contains("average candidate result allocation elapsed:"));
    assert!(!output.contains("average vector clone collection elapsed:"));
    assert!(!output.contains("Target workload resultless timing estimate"));
    assert!(!output.contains("average resultless retained elapsed:"));
    assert!(!output.contains("estimated result ownership overhead:"));
    assert!(!output.contains("estimated resultless retained share:"));
    assert!(!output.contains("Target workload count-only comparison"));
    assert!(!output.contains("owned average elapsed:"));
    assert!(!output.contains("count-only average elapsed:"));
    assert!(!output.contains("estimated owned result overhead:"));
    assert!(!output.contains("count-only speedup:"));
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

fn large_debug_cli_config() -> BenchmarkCliConfig {
    let mut config = debug_cli_config();

    config.suite_config = large_fast_suite_config();

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

fn large_fast_suite_config() -> BenchmarkSuiteConfig {
    let mut config = BenchmarkSuiteConfig::default();

    config.dataset_kind = BenchmarkDatasetKind::LargeClustered2D;
    config.timing_iterations = 1;
    config.max_leaf_size = 8;
    config.max_depth = 8;

    config
}
