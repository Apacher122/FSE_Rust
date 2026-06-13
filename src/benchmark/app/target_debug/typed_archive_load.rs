//! Typed archive load diagnostics.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::super::context::BenchmarkApplicationContext;
use super::super::renderer::BenchmarkApplicationRenderer;
use super::TypedQueryIndexArchiveArtifactValidation;
use super::formatting::format_speedup_ratio;
use super::target::{
    append_debug_duration_line, append_debug_line, append_target_workload_debug_section,
};
use super::typed_workload::{TypedBenchmarkContext, typed_x_range_plan};
use crate::benchmark::reports::output::format_duration_ascii;
use crate::benchmark::reports::{
    TypedArchiveAppendRebuildTimingReport, TypedArchiveCompactionTimingReport,
    TypedArchiveLoadTimingError, TypedArchiveLoadTimingReport,
    compare_typed_archive_append_rebuild_execution_repeated,
    compare_typed_archive_compaction_execution_repeated,
    compare_typed_archive_load_execution_repeated,
};
use crate::benchmark::workloads::QueryWorkloadCase;
use crate::build::FSEBuilder;
use crate::data::RowId;
use crate::persistence::{
    FSE_ARCHIVE_FILE_EXTENSION, load_typed_query_index_archive_file,
    save_typed_query_index_archive_file,
};

static ARCHIVE_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);
const COMPACTION_TOMBSTONE_FRACTION_DENOMINATOR: usize = 4;
const COMPACTION_TOMBSTONE_LIMIT: usize = 64;

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

    pub(crate) fn append_target_workload_typed_archive_append_rebuild_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        let typed_context = TypedBenchmarkContext::from_benchmark_context(context);

        append_target_workload_debug_section(
            output,
            context,
            "Target workload typed archive append rebuild timing",
            |output, context, workload| {
                let report = typed_archive_append_rebuild_report(context, &typed_context, workload);

                append_target_typed_archive_append_rebuild_report(output, &report);
            },
        );
    }

    pub(crate) fn append_workload_typed_archive_append_rebuild_summary_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        let typed_context = TypedBenchmarkContext::from_benchmark_context(context);

        output.push_str("Workload typed archive append rebuild timing summary\n");
        output.push_str("---------------------------------------------------\n");
        output.push_str(
            "workload | base records | appended records | resulting records | before bytes | after bytes | byte growth | matched after append | append rebuild | agreement\n",
        );

        for workload in &context.workloads {
            let report = typed_archive_append_rebuild_report(context, &typed_context, workload);

            output.push_str(&format!(
                "{} | {} | {} | {} | {} | {} | {} | {} | {} | pass\n",
                workload.name,
                report.base_record_count,
                report.appended_record_count,
                report.resulting_record_count,
                report.archive_bytes_before_append,
                report.archive_bytes_after_append,
                report.archive_byte_growth,
                report.matched_records_after_append,
                format_duration_ascii(report.append_rebuild_timing.average_elapsed),
            ));
        }

        output.push('\n');
    }

    pub(crate) fn append_target_workload_typed_archive_compaction_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        let typed_context = TypedBenchmarkContext::from_benchmark_context(context);

        append_target_workload_debug_section(
            output,
            context,
            "Target workload typed archive compaction timing",
            |output, context, workload| {
                let report = typed_archive_compaction_report(context, &typed_context, workload);

                append_target_typed_archive_compaction_report(output, &report);
            },
        );
    }

    pub(crate) fn append_workload_typed_archive_compaction_summary_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        let typed_context = TypedBenchmarkContext::from_benchmark_context(context);

        output.push_str("Workload typed archive compaction timing summary\n");
        output.push_str("-----------------------------------------------\n");
        output.push_str(
            "workload | base records | tombstones | removed records | retained records | index before bytes | index after bytes | index byte delta | tombstone before bytes | tombstone after bytes | tombstone byte delta | matched after compaction | compaction | agreement\n",
        );

        for workload in &context.workloads {
            let report = typed_archive_compaction_report(context, &typed_context, workload);

            output.push_str(&format!(
                "{} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | pass\n",
                workload.name,
                report.base_record_count,
                report.tombstone_count,
                report.removed_record_count,
                report.retained_record_count,
                report.query_archive_bytes_before_compaction,
                report.query_archive_bytes_after_compaction,
                report.query_archive_byte_delta,
                report.tombstone_archive_bytes_before_compaction,
                report.tombstone_archive_bytes_after_compaction,
                report.tombstone_archive_byte_delta,
                report.matched_records_after_compaction,
                format_duration_ascii(report.compaction_timing.average_elapsed),
            ));
        }

        output.push('\n');
    }
}

pub(super) fn write_validated_typed_query_index_archive_artifact<P>(
    context: &BenchmarkApplicationContext,
    path: P,
) -> Result<TypedQueryIndexArchiveArtifactValidation, TypedArchiveLoadTimingError>
where
    P: AsRef<Path>,
{
    let typed_context = TypedBenchmarkContext::from_benchmark_context(context);
    let path = path.as_ref();

    save_typed_query_index_archive_file(path, typed_context.query_index())?;

    let loaded_index = load_typed_query_index_archive_file(path)?;
    let mut matched_records = 0;

    for workload in &context.workloads {
        let plan = typed_x_range_plan(&typed_context, workload);
        let expected = typed_context.query_index().query_row_ids(&plan)?;
        let actual = loaded_index.query_row_ids(&plan)?;

        matched_records += expected.len();

        validate_same_row_id_set(
            "in-memory typed benchmark index",
            &expected,
            "emitted typed query index archive",
            &actual,
        )?;
    }

    Ok(TypedQueryIndexArchiveArtifactValidation {
        workloads_validated: context.workloads.len(),
        matched_records,
    })
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

fn append_target_typed_archive_append_rebuild_report(
    output: &mut String,
    report: &TypedArchiveAppendRebuildTimingReport,
) {
    append_debug_line(output, "base records", report.base_record_count);
    append_debug_line(output, "appended records", report.appended_record_count);
    append_debug_line(output, "resulting records", report.resulting_record_count);
    append_debug_line(
        output,
        "archive bytes before append",
        report.archive_bytes_before_append,
    );
    append_debug_line(
        output,
        "archive bytes after append",
        report.archive_bytes_after_append,
    );
    append_debug_line(output, "archive byte growth", report.archive_byte_growth);
    append_debug_line(
        output,
        "matched records after append",
        report.matched_records_after_append,
    );
    append_debug_duration_line(
        output,
        "append rebuild average elapsed",
        report.append_rebuild_timing.average_elapsed,
    );
    append_debug_line(output, "typed archive append rebuild agreement", "pass");
}

fn append_target_typed_archive_compaction_report(
    output: &mut String,
    report: &TypedArchiveCompactionTimingReport,
) {
    append_debug_line(output, "base records", report.base_record_count);
    append_debug_line(output, "tombstones", report.tombstone_count);
    append_debug_line(output, "removed records", report.removed_record_count);
    append_debug_line(output, "retained records", report.retained_record_count);
    append_debug_line(
        output,
        "query archive bytes before compaction",
        report.query_archive_bytes_before_compaction,
    );
    append_debug_line(
        output,
        "query archive bytes after compaction",
        report.query_archive_bytes_after_compaction,
    );
    append_debug_line(
        output,
        "query archive byte delta",
        report.query_archive_byte_delta,
    );
    append_debug_line(
        output,
        "tombstone archive bytes before compaction",
        report.tombstone_archive_bytes_before_compaction,
    );
    append_debug_line(
        output,
        "tombstone archive bytes after compaction",
        report.tombstone_archive_bytes_after_compaction,
    );
    append_debug_line(
        output,
        "tombstone archive byte delta",
        report.tombstone_archive_byte_delta,
    );
    append_debug_line(
        output,
        "matched records after compaction",
        report.matched_records_after_compaction,
    );
    append_debug_duration_line(
        output,
        "compaction average elapsed",
        report.compaction_timing.average_elapsed,
    );
    append_debug_line(output, "typed archive compaction agreement", "pass");
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

fn typed_archive_append_rebuild_report(
    context: &BenchmarkApplicationContext,
    typed_context: &TypedBenchmarkContext,
    workload: &QueryWorkloadCase,
) -> TypedArchiveAppendRebuildTimingReport {
    let plan = typed_x_range_plan(typed_context, workload);
    let path = typed_archive_temporary_path(workload);
    let appended = typed_context.append_batch_from_benchmark_context(context);
    let encoder = typed_context.encoder();
    let builder = FSEBuilder::new(context.suite_config.build_config());
    let report = compare_typed_archive_append_rebuild_execution_repeated(
        &path,
        typed_context.query_index(),
        &appended,
        &encoder,
        &builder,
        &plan,
        &context.timing_config,
    );

    let _ = fs::remove_file(&path);

    report.expect("typed archive append rebuild timing should execute")
}

fn typed_archive_compaction_report(
    context: &BenchmarkApplicationContext,
    typed_context: &TypedBenchmarkContext,
    workload: &QueryWorkloadCase,
) -> TypedArchiveCompactionTimingReport {
    let plan = typed_x_range_plan(typed_context, workload);
    let query_index_path = typed_archive_temporary_path(workload);
    let tombstone_path = typed_tombstone_archive_temporary_path(workload);
    let tombstone_row_ids = typed_archive_compaction_tombstones(typed_context);
    let encoder = typed_context.encoder();
    let builder = FSEBuilder::new(context.suite_config.build_config());
    let report = compare_typed_archive_compaction_execution_repeated(
        &query_index_path,
        &tombstone_path,
        typed_context.query_index(),
        &tombstone_row_ids,
        &encoder,
        &builder,
        &plan,
        &context.timing_config,
    );

    let _ = fs::remove_file(&query_index_path);
    let _ = fs::remove_file(&tombstone_path);

    report.expect("typed archive compaction timing should execute")
}

fn typed_archive_compaction_tombstones(typed_context: &TypedBenchmarkContext) -> Vec<RowId> {
    let row_ids = typed_context.query_index().batch().row_ids();
    let retained_record_guard = row_ids.len().saturating_sub(1);
    let tombstone_count = (row_ids.len() / COMPACTION_TOMBSTONE_FRACTION_DENOMINATOR)
        .max(1)
        .min(COMPACTION_TOMBSTONE_LIMIT)
        .min(retained_record_guard);

    row_ids.iter().copied().take(tombstone_count).collect()
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

fn typed_tombstone_archive_temporary_path(workload: &QueryWorkloadCase) -> PathBuf {
    let path_id = ARCHIVE_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
    let file_name = format!(
        "fse-typed-tombstone-archive-{}-{}-{}{}",
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

fn validate_same_row_id_set(
    expected_source: &'static str,
    expected: &[RowId],
    actual_source: &'static str,
    actual: &[RowId],
) -> Result<(), TypedArchiveLoadTimingError> {
    let mut expected = expected.to_vec();
    let mut actual = actual.to_vec();

    expected.sort_unstable();
    actual.sort_unstable();

    if expected == actual {
        return Ok(());
    }

    Err(TypedArchiveLoadTimingError::ResultMismatch {
        expected_source,
        actual_source,
        expected,
        actual,
    })
}
