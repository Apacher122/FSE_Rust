//! Terminal rendering for benchmark application output.

use super::context::BenchmarkApplicationContext;
use super::result_bundle::BenchmarkApplicationResultBundle;
use crate::benchmark::reports::{
    render_benchmark_overview, render_multi_baseline_summary, render_named_baseline_suite_report,
    render_suite_report,
};

/// Terminal renderer for benchmark application output.
///
/// # Runtime Role
///
/// `BenchmarkApplicationRenderer` owns terminal rendering for completed
/// benchmark application results. It keeps formatting decisions separate from
/// benchmark setup, execution, and CSV output.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BenchmarkApplicationRenderer;

impl BenchmarkApplicationRenderer {
    /// Creates a benchmark application renderer.
    pub fn new() -> Self {
        Self
    }

    /// Renders terminal output for a completed benchmark application run.
    pub fn render_terminal_output(
        &self,
        context: &BenchmarkApplicationContext,
        result_bundle: &BenchmarkApplicationResultBundle,
    ) -> String {
        let mut output = String::new();

        output.push_str(&render_benchmark_overview(&result_bundle.overview));
        self.append_suite_terminal_output(&mut output, context, result_bundle);

        output
    }

    fn append_suite_terminal_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
        result_bundle: &BenchmarkApplicationResultBundle,
    ) {
        if context.has_multiple_baselines() {
            self.append_multi_baseline_terminal_output(output, result_bundle);
        } else {
            self.append_single_baseline_terminal_output(output, result_bundle);
        }
    }

    fn append_single_baseline_terminal_output(
        &self,
        output: &mut String,
        result_bundle: &BenchmarkApplicationResultBundle,
    ) {
        // single baseline keeps the old compact output path
        output.push_str(&render_suite_report(
            &result_bundle.report.baseline_reports[0].report,
        ));
    }

    fn append_multi_baseline_terminal_output(
        &self,
        output: &mut String,
        result_bundle: &BenchmarkApplicationResultBundle,
    ) {
        for baseline_report in &result_bundle.report.baseline_reports {
            output.push_str(&render_named_baseline_suite_report(
                &baseline_report.baseline_name,
                &baseline_report.report,
            ));
        }

        // multi baseline gets the rollup after each named section
        output.push_str(&render_multi_baseline_summary(
            &result_bundle.aggregate_summary,
        ));
    }
}

/// Renders terminal output for a completed benchmark application run.
///
/// # Runtime Role
///
/// This helper preserves the previous function-level API while delegating
/// terminal formatting to `BenchmarkApplicationRenderer`.
pub fn render_benchmark_application_terminal_output(
    context: &BenchmarkApplicationContext,
    result_bundle: &BenchmarkApplicationResultBundle,
) -> String {
    BenchmarkApplicationRenderer::new().render_terminal_output(context, result_bundle)
}
