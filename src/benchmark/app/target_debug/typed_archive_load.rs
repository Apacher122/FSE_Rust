//! Typed archive load diagnostics.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::super::context::BenchmarkApplicationContext;
use super::super::renderer::BenchmarkApplicationRenderer;
use super::formatting::format_speedup_ratio;
use super::target::{
    append_debug_duration_line, append_debug_line, append_target_workload_debug_section,
};
use super::typed_workload::{TypedBenchmarkContext, typed_x_range_plan};
use crate::benchmark::reports::output::format_duration_ascii;
use crate::benchmark::reports::{
    TypedArchiveLoadTimingReport, compare_typed_archive_load_execution_repeated,
};
use crate::benchmark::workloads::QueryWorkloadCase;
use crate::persistence::{
    FSE_ARCHIVE_FILE_EXTENSION, FSETypedQueryIndexArchiveError, save_typed_query_index_archive_file,
};

static ARCHIVE_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

impl BenchmarkApplicationRenderer {
    pub(crate) fn append_target_workload_typed_archive_load_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        let typed_context = TypedBenchmarkContext::from_benchmark_context(context);

        append_target_workload_debug_section(
            output,
            context,
            "Target workload typed archive load timing",
            |output, context, workload| {
                let report = typed_archive_load_report(context, &typed_context, workload);

                append_target_typed_archive_load_report(output, &report);
            },
        );
    }

    pub(crate) fn append_workload_typed_archive_load_summary_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        let typed_context = TypedBenchmarkContext::from_benchmark_context(context);

        output.push_str("Workload typed archive load timing summary\n");
        output.push_str("------------------------------------------\n");
        output.push_str(
            "workload | matched | in-memory | warm-loaded | cold-loaded | warm/in-memory | cold/in-memory | cold/warm | agreement\n",
        );

        for workload in &context.workloads {
            let report = typed_archive_load_report(context, &typed_context, workload);

            output.push_str(&format!(
                "{} | {} | {} | {} | {} | {} | {} | {} | pass\n",
                workload.name,
                report.matched_records,
                format_duration_ascii(report.in_memory_timing.average_elapsed),
                format_duration_ascii(report.warm_loaded_timing.average_elapsed),
                format_duration_ascii(report.cold_loaded_timing.average_elapsed),
                format_speedup_ratio(report.warm_loaded_to_in_memory_ratio),
                format_speedup_ratio(report.cold_loaded_to_in_memory_ratio),
                format_speedup_ratio(report.cold_loaded_to_warm_loaded_ratio),
            ));
        }

        output.push('\n');
    }
}

pub(super) fn write_typed_query_index_archive_artifact<P>(
    context: &BenchmarkApplicationContext,
    path: P,
) -> Result<(), FSETypedQueryIndexArchiveError>
where
    P: AsRef<Path>,
{
    let typed_context = TypedBenchmarkContext::from_benchmark_context(context);

    save_typed_query_index_archive_file(path, typed_context.query_index())
}

fn append_target_typed_archive_load_report(
    output: &mut String,
    report: &TypedArchiveLoadTimingReport,
) {
    append_debug_line(output, "matched records", report.matched_records);
    append_debug_duration_line(
        output,
        "in-memory typed average elapsed",
        report.in_memory_timing.average_elapsed,
    );
    append_debug_duration_line(
        output,
        "warm-loaded typed average elapsed",
        report.warm_loaded_timing.average_elapsed,
    );
    append_debug_duration_line(
        output,
        "cold-loaded typed average elapsed",
        report.cold_loaded_timing.average_elapsed,
    );
    append_debug_line(
        output,
        "warm-loaded to in-memory ratio",
        format_speedup_ratio(report.warm_loaded_to_in_memory_ratio),
    );
    append_debug_line(
        output,
        "cold-loaded to in-memory ratio",
        format_speedup_ratio(report.cold_loaded_to_in_memory_ratio),
    );
    append_debug_line(
        output,
        "cold-loaded to warm-loaded ratio",
        format_speedup_ratio(report.cold_loaded_to_warm_loaded_ratio),
    );
    append_debug_line(output, "typed archive load agreement", "pass");
}

fn typed_archive_load_report(
    context: &BenchmarkApplicationContext,
    typed_context: &TypedBenchmarkContext,
    workload: &QueryWorkloadCase,
) -> TypedArchiveLoadTimingReport {
    let plan = typed_x_range_plan(typed_context, workload);
    let path = typed_archive_temporary_path(workload);
    let report = compare_typed_archive_load_execution_repeated(
        &path,
        typed_context.query_index(),
        &plan,
        &context.timing_config,
    );

    let _ = fs::remove_file(&path);

    report.expect("typed archive load timing should execute")
}

fn typed_archive_temporary_path(workload: &QueryWorkloadCase) -> PathBuf {
    let path_id = ARCHIVE_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
    let file_name = format!(
        "fse-typed-archive-load-{}-{}-{}{}",
        std::process::id(),
        path_id,
        sanitize_workload_name(&workload.name),
        FSE_ARCHIVE_FILE_EXTENSION
    );

    std::env::temp_dir().join(file_name)
}

fn sanitize_workload_name(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}
