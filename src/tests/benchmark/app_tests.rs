use std::fs;
use std::io::{self, Write};

use crate::benchmark::{
    BaselineKind, BenchmarkApplicationContext, BenchmarkApplicationOutput,
    BenchmarkApplicationOutputWriter, BenchmarkApplicationRenderer,
    BenchmarkApplicationResultBundle, BenchmarkBaselineSet, BenchmarkCliConfig,
    BenchmarkCsvOutputConfig, BenchmarkDatasetKind, BenchmarkSuiteConfig,
    BenchmarkTerminalOutputMode, render_benchmark_application_terminal_output,
    run_benchmark_application,
};
use crate::persistence::{FSE_ARCHIVE_FILE_EXTENSION, inspect_archive_payload};
use crate::query::QueryExecutionMode;

#[test]
fn benchmark_application_output_reports_empty_state() {
    let output = BenchmarkApplicationOutput::default();

    assert!(output.is_empty());
}

#[test]
fn benchmark_application_output_reports_non_empty_terminal_output() {
    let output = BenchmarkApplicationOutput::new("benchmark output".to_string(), Vec::new());

    assert!(!output.is_empty());
}

#[test]
fn benchmark_application_output_writer_writes_terminal_output() {
    let output = BenchmarkApplicationOutput::new("benchmark output".to_string(), Vec::new());
    let writer = BenchmarkApplicationOutputWriter::new();
    let mut buffer = Vec::new();

    writer.write(&output, &mut buffer).unwrap();

    assert_eq!(String::from_utf8(buffer).unwrap(), "benchmark output");
}

#[test]
fn benchmark_application_output_writer_writes_status_lines_after_terminal_output() {
    let output = BenchmarkApplicationOutput::new(
        "benchmark output\n".to_string(),
        vec![
            "CSV summary written: summary.csv".to_string(),
            "Workload CSV written: workloads.csv".to_string(),
        ],
    );
    let writer = BenchmarkApplicationOutputWriter::new();
    let mut buffer = Vec::new();

    writer.write(&output, &mut buffer).unwrap();

    assert_eq!(
        String::from_utf8(buffer).unwrap(),
        [
            "benchmark output",
            "CSV summary written: summary.csv",
            "Workload CSV written: workloads.csv",
            "",
        ]
        .join("\n")
    );
}

#[test]
fn benchmark_application_output_writer_writes_empty_output() {
    let output = BenchmarkApplicationOutput::default();
    let writer = BenchmarkApplicationOutputWriter::new();
    let mut buffer = Vec::new();

    // empty output should stay quiet not even a trailing newline
    writer.write(&output, &mut buffer).unwrap();

    assert!(buffer.is_empty());
}

#[test]
fn benchmark_application_output_writer_returns_terminal_write_error() {
    let output = BenchmarkApplicationOutput::new("benchmark output".to_string(), Vec::new());
    let writer = BenchmarkApplicationOutputWriter::new();
    let mut failing_writer = AlwaysFailingWriter;

    let result = writer.write(&output, &mut failing_writer);

    assert!(result.is_err());
}

#[test]
fn benchmark_application_output_writer_returns_status_line_write_error() {
    let terminal_output = "benchmark output\n";
    let output = BenchmarkApplicationOutput::new(
        terminal_output.to_string(),
        vec!["CSV summary written: summary.csv".to_string()],
    );
    let writer = BenchmarkApplicationOutputWriter::new();
    let mut failing_writer = ByteLimitedWriter::new(terminal_output.len());

    let result = writer.write(&output, &mut failing_writer);

    assert!(result.is_err());
    assert_eq!(failing_writer.contents(), terminal_output.as_bytes());
}

#[test]
fn benchmark_application_context_builds_single_baseline_context() {
    let context = BenchmarkApplicationContext::from_cli_config(single_baseline_cli_config());

    assert_eq!(
        context.baseline_set,
        BenchmarkBaselineSet::Single(BaselineKind::FlatScan)
    );
    assert_eq!(context.baseline_kinds, vec![BaselineKind::FlatScan]);
    assert_eq!(
        context.terminal_output_mode,
        BenchmarkTerminalOutputMode::Summary
    );
    assert!(!context.points.is_empty());
    assert!(!context.workloads.is_empty());
    assert!(context.index.node_count() > 0);
    assert!(context.validation.is_valid());
    assert!(!context.has_multiple_baselines());
    assert!(!context.uses_debug_report());
}

#[test]
fn benchmark_application_context_builds_debug_report_context() {
    let context =
        BenchmarkApplicationContext::from_cli_config(all_exact_baselines_debug_cli_config());

    assert_eq!(
        context.terminal_output_mode,
        BenchmarkTerminalOutputMode::DebugReport
    );
    assert!(context.uses_debug_report());
}

#[test]
fn benchmark_application_context_builds_all_exact_baselines_context() {
    let context = BenchmarkApplicationContext::from_cli_config(all_exact_baselines_cli_config());

    assert_eq!(context.baseline_set, BenchmarkBaselineSet::AllExact);
    assert_eq!(
        context.baseline_kinds,
        vec![
            BaselineKind::FlatScan,
            BaselineKind::KdTree,
            BaselineKind::RTree,
        ]
    );
    assert!(context.has_multiple_baselines());
}

#[test]
fn benchmark_application_context_builds_overview_from_current_state() {
    let context = BenchmarkApplicationContext::from_cli_config(single_baseline_cli_config());
    let overview = context.overview();

    assert_eq!(overview.dataset_records, context.points.len());
    assert_eq!(overview.index_nodes, context.index.node_count());
    assert_eq!(overview.workloads, context.workloads.len());
    assert_eq!(overview.baselines, "flat_scan");
    assert_eq!(overview.timing_iterations, context.timing_config.iterations);
    assert_eq!(overview.max_leaf_size, context.suite_config.max_leaf_size);
    assert_eq!(overview.max_depth, context.suite_config.max_depth);
    assert_eq!(
        overview.fse_execution_mode,
        context.suite_config.fse_execution_mode
    );
    assert_eq!(
        overview.fse_parallel_min_retained_leaves,
        context.suite_config.fse_parallel_min_retained_leaves
    );
    assert_eq!(overview.validation, context.validation);
}

#[test]
fn benchmark_application_context_builds_parallel_overview_from_current_state() {
    let context = BenchmarkApplicationContext::from_cli_config(parallel_baseline_cli_config());
    let overview = context.overview();

    assert_eq!(overview.fse_execution_mode, QueryExecutionMode::Parallel);
    assert_eq!(overview.fse_execution_mode_name(), "parallel");
    assert_eq!(overview.fse_parallel_min_retained_leaves, 2);
}

#[test]
fn benchmark_application_context_runs_suite() {
    let context = BenchmarkApplicationContext::from_cli_config(single_baseline_cli_config());
    let report = context.run_suite();

    // this is wiring coverage not benchmark accuracy
    assert_eq!(report.baseline_reports.len(), 1);
    assert_eq!(report.baseline_reports[0].baseline_name, "flat_scan");
}

#[test]
fn benchmark_application_result_bundle_matches_context_state() {
    let context = BenchmarkApplicationContext::from_cli_config(single_baseline_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);

    assert_eq!(result_bundle.overview.dataset_records, context.points.len());
    assert_eq!(
        result_bundle.overview.index_nodes,
        context.index.node_count()
    );
    assert_eq!(result_bundle.overview.workloads, context.workloads.len());
    assert_eq!(
        result_bundle.overview.fse_execution_mode,
        context.suite_config.fse_execution_mode
    );
    assert_eq!(
        result_bundle.overview.fse_parallel_min_retained_leaves,
        context.suite_config.fse_parallel_min_retained_leaves
    );
    assert_eq!(result_bundle.baseline_report_count(), 1);
    assert_eq!(result_bundle.metadata.dataset_records, context.points.len());
    assert_eq!(
        result_bundle.metadata.selected_baselines,
        context.baseline_set.selected_name_list()
    );
}

#[test]
fn benchmark_application_result_bundle_keeps_all_exact_baseline_reports() {
    let context = BenchmarkApplicationContext::from_cli_config(all_exact_baselines_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);

    assert_eq!(result_bundle.baseline_report_count(), 3);
}

#[test]
fn benchmark_application_renderer_uses_compact_summary_by_default() {
    let context = BenchmarkApplicationContext::from_cli_config(all_exact_baselines_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(output.contains("FSE Benchmark Summary"));
    assert!(output.contains("FSE execution: serial"));
    assert!(output.contains("FSE parallel min leaves"));
    assert!(output.contains("Result"));
    assert!(output.contains("Best relative result"));
    assert!(output.contains("Diagnosis"));
    assert!(output.contains("Next target"));
    assert!(output.contains("flat_scan"));
    assert!(output.contains("kd_tree"));
    assert!(output.contains("r_tree"));
    assert!(!output.contains("Workload: cluster_range_000"));
    assert!(!output.contains("Selectivity Bucket Summary"));
}

#[test]
fn benchmark_application_renderer_renders_parallel_mode_in_compact_summary() {
    let context = BenchmarkApplicationContext::from_cli_config(parallel_baseline_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(output.contains("FSE execution: parallel"));
    assert!(output.contains("FSE parallel min leaves: 2"));
}

#[test]
fn benchmark_application_renderer_uses_detailed_output_for_debug_report() {
    let context =
        BenchmarkApplicationContext::from_cli_config(all_exact_baselines_debug_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(output.contains("FSE benchmark suite"));
    assert!(output.contains("FSE execution: serial"));
    assert!(output.contains("FSE parallel min leaves"));
    assert!(output.contains("Workload: cluster_range_000"));
    assert!(output.contains("Selectivity Bucket Summary"));
    assert!(output.contains("Workload materialization mode summary"));
    assert!(output.contains("Workload exact existence timing summary"));
    assert!(output.contains("Target workload exact existence timing"));
    assert!(output.contains("Target workload typed indexed comparison"));
    assert!(output.contains("Target workload typed archive load timing"));
    assert!(output.contains("archive bytes"));
    assert!(output.contains("row-mapped index section bytes"));
    assert!(output.contains("typed record batch section bytes"));
    assert!(output.contains("record encoder metadata section bytes"));
    assert!(output.contains("Target workload typed archive append rebuild timing"));
    assert!(output.contains("Target workload typed archive append-delta query timing"));
    assert!(output.contains("Target workload typed archive compaction timing"));
    assert!(output.contains("Target workload typed archive maintenance timing"));
    assert!(output.contains("Target workload typed archive append-delta maintenance timing"));
    assert!(output.contains("logical archive bytes before compaction"));
    assert!(output.contains("append-delta reconstructed records"));
    assert!(output.contains("rebuilt query average elapsed"));
    assert!(output.contains("planner strategy"));
    assert!(output.contains("planner reason"));
    assert!(output.contains("planner cost classification"));
    assert!(output.contains("hierarchy metadata bytes"));
    assert!(output.contains("records pruned"));
    assert!(output.contains("planner selectivity bucket"));
    assert!(output.contains("planner risk"));
    assert!(output.contains("planner predicate evaluation delta vs flat scan"));
    assert!(output.contains("planner flat scan record delta vs flat scan"));
    assert!(output.contains("planner traversal node visit delta vs flat scan"));
    assert!(output.contains("baseline typed existence has match"));
    assert!(output.contains("planned typed existence has match"));
    assert!(output.contains("typed scan existence average elapsed"));
    assert!(output.contains("planned typed existence average elapsed"));
    assert!(output.contains("existence planner strategy"));
    assert!(output.contains("existence planner reason"));
    assert!(output.contains("existence planner cost classification"));
    assert!(output.contains("typed existence comparison agreement"));
    assert!(output.contains("logical archive bytes before maintenance"));
    assert!(output.contains("append archive bytes before maintenance"));
    assert!(output.contains("maintenance status requires archive write"));
    assert!(output.contains("Workload typed indexed comparison summary"));
    assert!(output.contains("Workload typed indexed existence comparison summary"));
    assert!(output.contains("Workload typed archive load timing summary"));
    assert!(output.contains("Workload typed archive append rebuild timing summary"));
    assert!(output.contains("Workload typed archive append-delta query timing summary"));
    assert!(output.contains("Workload typed archive compaction timing summary"));
    assert!(output.contains("Workload typed archive maintenance timing summary"));
    assert!(output.contains("Workload typed archive append-delta maintenance timing summary"));
}

#[test]
fn benchmark_application_renderer_renders_single_baseline_summary_output() {
    let context = BenchmarkApplicationContext::from_cli_config(single_baseline_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(!output.is_empty());
    assert!(output.contains("FSE Benchmark Summary"));
    assert!(output.contains("FSE execution: serial"));
    assert!(output.contains("flat_scan"));
    assert!(!output.contains("Workload: cluster_range_000"));
    assert!(!output.contains("Selectivity Bucket Summary"));
}

#[test]
fn benchmark_application_renderer_renders_debug_selectivity_summary() {
    let context = BenchmarkApplicationContext::from_cli_config(single_baseline_debug_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(output.contains("Selectivity Bucket Summary"));
    assert!(output.contains("empty |"));
    assert!(output.contains("full |"));
}

#[test]
fn benchmark_application_renderer_renders_debug_selectivity_summary_for_each_exact_baseline() {
    let context =
        BenchmarkApplicationContext::from_cli_config(all_exact_baselines_debug_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert_eq!(output.matches("Selectivity Bucket Summary").count(), 3);
}

#[test]
fn benchmark_application_render_function_delegates_to_renderer() {
    let context = BenchmarkApplicationContext::from_cli_config(single_baseline_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let direct_output = renderer.render_terminal_output(&context, &result_bundle);
    let delegated_output = render_benchmark_application_terminal_output(&context, &result_bundle);

    assert_eq!(delegated_output, direct_output);
}

#[test]
fn benchmark_application_runs_single_baseline_configuration() {
    let output = run_benchmark_application(single_baseline_cli_config()).unwrap();

    assert!(!output.terminal_output.is_empty());
    assert!(output.terminal_output.contains("FSE Benchmark Summary"));
    assert!(output.terminal_output.contains("FSE execution: serial"));
    assert!(output.terminal_output.contains("flat_scan"));
    assert!(output.status_lines.is_empty());
}

#[test]
fn benchmark_application_runs_all_exact_baselines_configuration() {
    let output = run_benchmark_application(all_exact_baselines_cli_config()).unwrap();

    assert!(!output.terminal_output.is_empty());
    assert!(output.terminal_output.contains("FSE Benchmark Summary"));
    assert!(output.terminal_output.contains("FSE execution: serial"));
    assert!(output.terminal_output.contains("flat_scan"));
    assert!(output.terminal_output.contains("kd_tree"));
    assert!(output.terminal_output.contains("r_tree"));
    assert!(output.status_lines.is_empty());
}

#[test]
fn benchmark_application_runs_debug_report_configuration() {
    let output = run_benchmark_application(all_exact_baselines_debug_cli_config()).unwrap();

    assert!(!output.terminal_output.is_empty());
    assert!(output.terminal_output.contains("FSE benchmark suite"));
    assert!(output.terminal_output.contains("FSE execution: serial"));
    assert!(
        output
            .terminal_output
            .contains("Workload: cluster_range_000")
    );
    assert!(
        output
            .terminal_output
            .contains("Selectivity Bucket Summary")
    );
    assert!(
        output
            .terminal_output
            .contains("Target workload materialization mode comparison")
    );
    assert!(
        output
            .terminal_output
            .contains("Target workload exact existence timing")
    );
    assert!(
        output
            .terminal_output
            .contains("Target workload typed indexed comparison")
    );
    assert!(
        output
            .terminal_output
            .contains("Target workload typed archive load timing")
    );
    assert!(
        output
            .terminal_output
            .contains("Target workload typed archive append rebuild timing")
    );
    assert!(
        output
            .terminal_output
            .contains("Target workload typed archive append-delta query timing")
    );
    assert!(
        output
            .terminal_output
            .contains("Target workload typed archive compaction timing")
    );
    assert!(
        output
            .terminal_output
            .contains("Target workload typed archive maintenance timing")
    );
    assert!(
        output
            .terminal_output
            .contains("Target workload typed archive append-delta maintenance timing")
    );
    assert!(
        output
            .terminal_output
            .contains("logical archive bytes before compaction")
    );
    assert!(
        output
            .terminal_output
            .contains("logical archive bytes before maintenance")
    );
    assert!(
        output
            .terminal_output
            .contains("append archive bytes before maintenance")
    );
    assert!(
        output
            .terminal_output
            .contains("append-delta reconstructed records")
    );
    assert!(output.terminal_output.contains("planner strategy"));
    assert!(output.terminal_output.contains("planner reason"));
    assert!(
        output
            .terminal_output
            .contains("planner cost classification")
    );
    assert!(output.terminal_output.contains("hierarchy metadata bytes"));
    assert!(output.terminal_output.contains("records pruned"));
    assert!(
        output
            .terminal_output
            .contains("planner selectivity bucket")
    );
    assert!(output.terminal_output.contains("planner risk"));
    assert!(
        output
            .terminal_output
            .contains("planner predicate evaluation delta vs flat scan")
    );
    assert!(
        output
            .terminal_output
            .contains("planner flat scan record delta vs flat scan")
    );
    assert!(
        output
            .terminal_output
            .contains("planner traversal node visit delta vs flat scan")
    );
    assert!(
        output
            .terminal_output
            .contains("baseline typed existence has match")
    );
    assert!(
        output
            .terminal_output
            .contains("planned typed existence has match")
    );
    assert!(
        output
            .terminal_output
            .contains("typed scan existence average elapsed")
    );
    assert!(
        output
            .terminal_output
            .contains("planned typed existence average elapsed")
    );
    assert!(
        output
            .terminal_output
            .contains("existence planner strategy")
    );
    assert!(output.terminal_output.contains("existence planner reason"));
    assert!(
        output
            .terminal_output
            .contains("existence planner cost classification")
    );
    assert!(
        output
            .terminal_output
            .contains("typed existence comparison agreement")
    );
    assert!(output.terminal_output.contains("rebuilt/append-delta"));
    assert!(
        output
            .terminal_output
            .contains("Workload typed indexed comparison summary")
    );
    assert!(
        output
            .terminal_output
            .contains("Workload typed indexed existence comparison summary")
    );
    assert!(
        output
            .terminal_output
            .contains("Workload typed archive load timing summary")
    );
    assert!(
        output
            .terminal_output
            .contains("Workload typed archive append rebuild timing summary")
    );
    assert!(
        output
            .terminal_output
            .contains("Workload typed archive append-delta query timing summary")
    );
    assert!(
        output
            .terminal_output
            .contains("Workload typed archive compaction timing summary")
    );
    assert!(
        output
            .terminal_output
            .contains("Workload typed archive maintenance timing summary")
    );
    assert!(
        output
            .terminal_output
            .contains("Workload typed archive append-delta maintenance timing summary")
    );
    assert!(output.terminal_output.contains("status write required"));
    assert!(
        output
            .terminal_output
            .contains("cluster_boundary_range | 60 | 15 | 15 | 45")
    );
    assert!(
        output
            .terminal_output
            .contains("all matched records agree: true")
    );
    assert!(output.status_lines.is_empty());
}

#[test]
fn benchmark_application_writes_typed_query_index_archive_artifact() {
    let path = archive_path("benchmark_application_writes_typed_query_index_archive_artifact");
    let _ = fs::remove_file(&path);
    let mut config = single_baseline_cli_config();

    config.typed_query_index_archive_path = Some(path.to_string_lossy().into_owned());

    let output = run_benchmark_application(config).unwrap();

    assert!(path.exists());
    assert!(fs::metadata(&path).unwrap().len() > 0);
    let metadata = inspect_archive_payload(&fs::read(&path).unwrap()).unwrap();
    let path_display = path.to_string_lossy();
    assert_eq!(
        output.status_lines,
        vec![
            format!("Typed query index archive written: {}", path_display),
            format!(
                "Typed query index archive payload: {} (kind=typed_query_index, header_version={}, payload_length={}, payload_checksum={:#018x})",
                path_display,
                metadata.header_version,
                metadata.payload_length,
                metadata.payload_checksum
            ),
            format!(
                "Typed query index archive validated: {} (6 workloads, 83 matched records)",
                path_display
            )
        ]
    );

    fs::remove_file(path).expect("typed query index archive artifact should be removable");
}

struct AlwaysFailingWriter;

impl Write for AlwaysFailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "forced writer failure",
        ))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct ByteLimitedWriter {
    bytes_allowed: usize,
    buffer: Vec<u8>,
}

impl ByteLimitedWriter {
    fn new(bytes_allowed: usize) -> Self {
        Self {
            bytes_allowed,
            buffer: Vec::new(),
        }
    }

    fn contents(&self) -> &[u8] {
        &self.buffer
    }
}

impl Write for ByteLimitedWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self.buffer.len() + buffer.len() > self.bytes_allowed {
            return Err(io::Error::new(io::ErrorKind::Other, "forced writer limit"));
        }

        // this writer keeps successful bytes so the test can check where it failed
        self.buffer.extend_from_slice(buffer);

        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn single_baseline_cli_config() -> BenchmarkCliConfig {
    benchmark_cli_config(BenchmarkBaselineSet::Single(BaselineKind::FlatScan))
}

fn parallel_baseline_cli_config() -> BenchmarkCliConfig {
    let mut config = benchmark_cli_config(BenchmarkBaselineSet::Single(BaselineKind::FlatScan));

    config.suite_config = config
        .suite_config
        .with_fse_execution_mode(QueryExecutionMode::Parallel)
        .with_fse_parallel_min_retained_leaves(2);

    config
}

fn single_baseline_debug_cli_config() -> BenchmarkCliConfig {
    let mut config = benchmark_cli_config(BenchmarkBaselineSet::Single(BaselineKind::FlatScan));

    config.terminal_output_mode = BenchmarkTerminalOutputMode::DebugReport;

    config
}

fn all_exact_baselines_cli_config() -> BenchmarkCliConfig {
    benchmark_cli_config(BenchmarkBaselineSet::AllExact)
}

fn all_exact_baselines_debug_cli_config() -> BenchmarkCliConfig {
    let mut config = benchmark_cli_config(BenchmarkBaselineSet::AllExact);

    config.terminal_output_mode = BenchmarkTerminalOutputMode::DebugReport;

    config
}

fn benchmark_cli_config(baseline_set: BenchmarkBaselineSet) -> BenchmarkCliConfig {
    BenchmarkCliConfig {
        suite_config: small_fast_suite_config(),
        baseline_set,
        baseline_kinds: baseline_set.selected_kinds(),
        csv_output: BenchmarkCsvOutputConfig::default(),
        typed_query_index_archive_path: None,
        terminal_output_mode: BenchmarkTerminalOutputMode::Summary,
    }
}

fn small_fast_suite_config() -> BenchmarkSuiteConfig {
    let mut config = BenchmarkSuiteConfig::default();

    config.dataset_kind = BenchmarkDatasetKind::SmallClustered2D;
    config.timing_iterations = 1;

    // this test is about wiring not stable timing numbers
    config.max_leaf_size = 8;
    config.max_depth = 8;

    config
}

fn archive_path(test_name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "fse-{test_name}-{}{}",
        std::process::id(),
        FSE_ARCHIVE_FILE_EXTENSION
    ))
}
