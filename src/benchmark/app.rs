//! Benchmark application orchestration.
//!
//! This module connects CLI configuration, index construction, benchmark
//! execution, terminal rendering, aggregate summaries, and optional CSV output.

use std::fs;

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
use crate::persistence::{
    FSEArchivePayloadKind, FSEArchivePayloadMetadata, inspect_archive_payload,
};

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
    let status_lines = write_application_file_outputs(context, result_bundle)?;

    Ok(BenchmarkApplicationOutput::new(
        terminal_output,
        status_lines,
    ))
}

fn render_application_terminal_output(
    context: &BenchmarkApplicationContext,
    result_bundle: &BenchmarkApplicationResultBundle,
) -> String {
    // renderer is stateless so build it right where its used
    BenchmarkApplicationRenderer::new().render_terminal_output(context, result_bundle)
}

fn write_application_file_outputs(
    context: &BenchmarkApplicationContext,
    result_bundle: &BenchmarkApplicationResultBundle,
) -> Result<Vec<String>, BenchmarkApplicationError> {
    let mut status_lines = write_application_csv_outputs(context, result_bundle)?;

    if let Some(path) = &context.typed_query_index_archive_path {
        let validation = target_debug::write_validated_typed_query_index_archive_artifact(
            context,
            path.as_str(),
        )?;

        status_lines.push(format!("Typed query index archive written: {path}"));
        let metadata = inspect_typed_query_index_archive_payload(path)?;
        status_lines.push(format_typed_query_index_archive_payload_status(
            path, metadata,
        ));
        status_lines.push(format!(
            "Typed query index archive validated: {path} ({} workloads, {} matched records)",
            validation.workloads_validated, validation.matched_records
        ));
    }

    Ok(status_lines)
}

fn inspect_typed_query_index_archive_payload(
    path: &str,
) -> Result<FSEArchivePayloadMetadata, BenchmarkApplicationError> {
    let bytes = fs::read(path).map_err(|error| {
        BenchmarkApplicationError::TypedQueryIndexArchiveMetadataRead {
            path: path.to_string(),
            kind: error.kind(),
        }
    })?;

    inspect_archive_payload(&bytes)
        .map_err(BenchmarkApplicationError::TypedQueryIndexArchivePayload)
}

fn format_typed_query_index_archive_payload_status(
    path: &str,
    metadata: FSEArchivePayloadMetadata,
) -> String {
    format!(
        "Typed query index archive payload: {path} (kind={}, header_version={}, payload_length={}, payload_checksum={:#018x})",
        archive_payload_kind_name(metadata.kind),
        metadata.header_version,
        metadata.payload_length,
        metadata.payload_checksum
    )
}

fn archive_payload_kind_name(kind: FSEArchivePayloadKind) -> &'static str {
    match kind {
        FSEArchivePayloadKind::Index => "index",
        FSEArchivePayloadKind::RowMappedIndex => "row_mapped_index",
        FSEArchivePayloadKind::TypedRecordBatch => "typed_record_batch",
        FSEArchivePayloadKind::TypedQueryIndex => "typed_query_index",
        FSEArchivePayloadKind::TypedRowTombstone => "typed_row_tombstone",
    }
}

fn write_application_csv_outputs(
    context: &BenchmarkApplicationContext,
    result_bundle: &BenchmarkApplicationResultBundle,
) -> Result<Vec<String>, BenchmarkApplicationError> {
    let write_report = write_benchmark_csv_outputs(
        &context.csv_output,
        &result_bundle.metadata,
        &result_bundle.aggregate_summary,
        &result_bundle.report,
    )?;

    Ok(write_report.status_lines())
}
