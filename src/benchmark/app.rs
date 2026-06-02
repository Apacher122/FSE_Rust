//! Benchmark application orchestration.
//!
//! This module connects CLI configuration, index construction, benchmark
//! execution, terminal rendering, aggregate summaries, and optional CSV output.

pub mod context;
pub mod error;
pub mod output;
pub mod renderer;
pub mod result_bundle;
mod target_debug;

pub use context::BenchmarkApplicationContext;
pub use error::BenchmarkApplicationError;
pub use output::{BenchmarkApplicationOutput, BenchmarkApplicationOutputWriter};
pub use renderer::{BenchmarkApplicationRenderer, render_benchmark_application_terminal_output};
pub use result_bundle::BenchmarkApplicationResultBundle;

use crate::benchmark::cli::BenchmarkCliConfig;
use crate::benchmark::reports::write_benchmark_csv_outputs;

/// Runs a benchmark from parsed CLI configuration.
///
/// # Runtime Role
///
/// This function is the high-level application boundary for benchmark execution.
/// It owns the build, run, render, summary, and CSV-output sequence so the binary
/// entrypoint does not need to duplicate benchmark workflow details.
pub fn run_benchmark_application(
    cli_config: BenchmarkCliConfig,
) -> Result<BenchmarkApplicationOutput, BenchmarkApplicationError> {
    let context = BenchmarkApplicationContext::try_from_cli_config(cli_config)?;
    let result_bundle = BenchmarkApplicationResultBundle::from_context(&context);

    // keep the public run function as the short happy path
    build_benchmark_application_output(&context, &result_bundle)
}

fn build_benchmark_application_output(
    context: &BenchmarkApplicationContext,
    result_bundle: &BenchmarkApplicationResultBundle,
) -> Result<BenchmarkApplicationOutput, BenchmarkApplicationError> {
    let terminal_output = render_application_terminal_output(context, result_bundle);
    let csv_status_lines = write_application_csv_outputs(context, result_bundle)?;

    Ok(BenchmarkApplicationOutput::new(
        terminal_output,
        csv_status_lines,
    ))
}

fn render_application_terminal_output(
    context: &BenchmarkApplicationContext,
    result_bundle: &BenchmarkApplicationResultBundle,
) -> String {
    // renderer is stateless so build it right where its used
    BenchmarkApplicationRenderer::new().render_terminal_output(context, result_bundle)
}

fn write_application_csv_outputs(
    context: &BenchmarkApplicationContext,
    result_bundle: &BenchmarkApplicationResultBundle,
) -> Result<Vec<String>, BenchmarkApplicationError> {
    // csv is the only fallible output step right now
    let write_report = write_benchmark_csv_outputs(
        &context.csv_output,
        &result_bundle.metadata,
        &result_bundle.aggregate_summary,
        &result_bundle.report,
    )?;

    Ok(write_report.status_lines())
}
