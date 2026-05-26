//! Terminal rendering for benchmark application output.
//!
//! The renderer owns terminal output for completed benchmark application runs.
//! Summary output, debug output, and small formatting helpers live in separate
//! files so the renderer module can grow without becoming one large file.

mod debug;
mod helpers;
mod summary;

use super::context::BenchmarkApplicationContext;
use super::result_bundle::BenchmarkApplicationResultBundle;

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
        if context.uses_debug_report() {
            return self.render_debug_terminal_output(context, result_bundle);
        }

        self.render_summary_terminal_output(
            &result_bundle.overview,
            &result_bundle.aggregate_summary,
        )
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
