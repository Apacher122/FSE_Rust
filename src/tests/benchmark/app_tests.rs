use std::io::{self, Write};

use crate::benchmark::{
    BaselineKind, BenchmarkApplicationContext, BenchmarkApplicationOutput,
    BenchmarkApplicationOutputWriter, BenchmarkApplicationRenderer,
    BenchmarkApplicationResultBundle, BenchmarkBaselineSet, BenchmarkCliConfig,
    BenchmarkCsvOutputConfig, BenchmarkDatasetKind, BenchmarkSuiteConfig,
    render_benchmark_application_terminal_output, run_benchmark_application,
};

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
fn benchmark_application_output_writer_writes_csv_status_lines_after_terminal_output() {
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
    assert!(!context.points.is_empty());
    assert!(!context.workloads.is_empty());
    assert!(context.index.node_count() > 0);
    assert!(context.validation.is_valid());
    assert!(!context.has_multiple_baselines());
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
    assert_eq!(overview.validation, context.validation);
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
fn benchmark_application_renderer_renders_single_baseline_terminal_output() {
    let context = BenchmarkApplicationContext::from_cli_config(single_baseline_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(!output.is_empty());
    assert!(output.contains("flat_scan"));
}

#[test]
fn benchmark_application_renderer_renders_all_exact_baseline_terminal_output() {
    let context = BenchmarkApplicationContext::from_cli_config(all_exact_baselines_cli_config());
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);
    let renderer = BenchmarkApplicationRenderer::new();

    let output = renderer.render_terminal_output(&context, &result_bundle);

    assert!(!output.is_empty());
    assert!(output.contains("flat_scan"));
    assert!(output.contains("kd_tree"));
    assert!(output.contains("r_tree"));
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
    assert!(output.terminal_output.contains("flat_scan"));
    assert!(output.csv_status_lines.is_empty());
}

#[test]
fn benchmark_application_runs_all_exact_baselines_configuration() {
    let output = run_benchmark_application(all_exact_baselines_cli_config()).unwrap();

    assert!(!output.terminal_output.is_empty());
    assert!(output.terminal_output.contains("flat_scan"));
    assert!(output.terminal_output.contains("kd_tree"));
    assert!(output.terminal_output.contains("r_tree"));
    assert!(output.csv_status_lines.is_empty());
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
    let suite_config = small_fast_suite_config();
    let baseline_set = BenchmarkBaselineSet::Single(BaselineKind::FlatScan);

    BenchmarkCliConfig {
        suite_config,
        baseline_set,
        baseline_kinds: baseline_set.selected_kinds(),
        csv_output: BenchmarkCsvOutputConfig::default(),
    }
}

fn all_exact_baselines_cli_config() -> BenchmarkCliConfig {
    let suite_config = small_fast_suite_config();
    let baseline_set = BenchmarkBaselineSet::AllExact;

    BenchmarkCliConfig {
        suite_config,
        baseline_set,
        baseline_kinds: baseline_set.selected_kinds(),
        csv_output: BenchmarkCsvOutputConfig::default(),
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
