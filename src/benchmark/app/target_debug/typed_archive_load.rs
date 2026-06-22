//! Typed archive load diagnostics.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::super::context::BenchmarkApplicationContext;
use super::super::renderer::BenchmarkApplicationRenderer;
use super::TypedQueryIndexArchiveArtifactValidation;
use super::formatting::{format_percent_ratio, format_speedup_ratio};
use super::target::{
    append_debug_duration_line, append_debug_line, append_target_workload_debug_section,
};
use super::typed_workload::{TypedBenchmarkContext, typed_x_range_plan};
use crate::benchmark::reports::output::format_duration_ascii;
use crate::benchmark::reports::{
    TypedArchiveAppendDeltaMaintenanceTimingReport, TypedArchiveAppendDeltaQueryTimingReport,
    TypedArchiveAppendRebuildTimingReport, TypedArchiveCompactionTimingReport,
    TypedArchiveLoadTimingError, TypedArchiveLoadTimingReport, TypedArchiveMaintenanceTimingReport,
    compare_typed_archive_append_delta_maintenance_execution_repeated,
    compare_typed_archive_append_delta_query_execution_repeated,
    compare_typed_archive_append_rebuild_execution_repeated,
    compare_typed_archive_compaction_execution_repeated,
    compare_typed_archive_load_execution_repeated,
    compare_typed_archive_maintenance_execution_repeated,
};
use crate::benchmark::workloads::QueryWorkloadCase;
use crate::build::FSEBuilder;
use crate::data::RowId;
use crate::persistence::{
    FSE_ARCHIVE_FILE_EXTENSION, FSEArchiveMaintenanceAction, FSEArchiveMaintenancePolicy,
    FSEArchiveMaintenanceReason, load_typed_query_index_archive_file,
    save_typed_query_index_archive_file,
};

static ARCHIVE_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);
const COMPACTION_TOMBSTONE_FRACTION_DENOMINATOR: usize = 4;
const COMPACTION_TOMBSTONE_LIMIT: usize = 64;
const MAINTENANCE_APPEND_REBUILD_RECORD_COUNT_THRESHOLD: u64 = 10;
const MAINTENANCE_COMPACTION_TOMBSTONE_COUNT_THRESHOLD: u64 = 1;
const MAINTENANCE_COMPACTION_TOMBSTONE_RATIO_THRESHOLD_BASIS_POINTS: u64 = 9_000;

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

    pub(crate) fn append_target_workload_typed_archive_append_delta_query_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        let typed_context = TypedBenchmarkContext::from_benchmark_context(context);

        append_target_workload_debug_section(
            output,
            context,
            "Target workload typed archive append-delta query timing",
            |output, context, workload| {
                let report =
                    typed_archive_append_delta_query_report(context, &typed_context, workload);

                append_target_typed_archive_append_delta_query_report(output, &report);
            },
        );
    }

    pub(crate) fn append_workload_typed_archive_append_delta_query_summary_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        let typed_context = TypedBenchmarkContext::from_benchmark_context(context);

        output.push_str("Workload typed archive append-delta query timing summary\n");
        output.push_str("-------------------------------------------------------\n");
        output.push_str(
            "workload | base records | appended records | rebuilt records | append-delta matched | rebuilt matched | append-delta reconstructed | rebuilt reconstructed | append-delta candidate ratio | rebuilt candidate ratio | rebuilt/append-delta | rebuilt query | append-delta query | agreement\n",
        );

        for workload in &context.workloads {
            let report = typed_archive_append_delta_query_report(context, &typed_context, workload);

            output.push_str(&format!(
                "{} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | pass\n",
                workload.name,
                report.base_record_count,
                report.appended_record_count,
                report.rebuilt_record_count,
                report.append_delta_matched_records,
                report.rebuilt_matched_records,
                report.append_delta_stats.reconstructed_records,
                report.rebuilt_stats.reconstructed_records,
                format_scalar_percent(report.append_delta_stats.candidate_ratio),
                format_scalar_percent(report.rebuilt_stats.candidate_ratio),
                format_speedup_ratio(report.rebuilt_to_append_delta_average_ratio),
                format_duration_ascii(report.repeated_timing.baseline.average_elapsed),
                format_duration_ascii(report.repeated_timing.fse.average_elapsed),
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
            "workload | base records | tombstones | removed records | retained records | index before bytes | index after bytes | index byte delta | tombstone before bytes | tombstone after bytes | tombstone byte delta | logical before bytes | logical after bytes | logical byte delta | matched after compaction | compaction | agreement\n",
        );

        for workload in &context.workloads {
            let report = typed_archive_compaction_report(context, &typed_context, workload);

            output.push_str(&format!(
                "{} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | pass\n",
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
                report.logical_archive_bytes_before_compaction,
                report.logical_archive_bytes_after_compaction,
                report.logical_archive_byte_delta,
                report.matched_records_after_compaction,
                format_duration_ascii(report.compaction_timing.average_elapsed),
            ));
        }

        output.push('\n');
    }

    pub(crate) fn append_target_workload_typed_archive_maintenance_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        let typed_context = TypedBenchmarkContext::from_benchmark_context(context);

        append_target_workload_debug_section(
            output,
            context,
            "Target workload typed archive maintenance timing",
            |output, context, workload| {
                let report = typed_archive_maintenance_report(context, &typed_context, workload);

                append_target_typed_archive_maintenance_report(output, &report);
            },
        );
    }

    pub(crate) fn append_workload_typed_archive_maintenance_summary_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        let typed_context = TypedBenchmarkContext::from_benchmark_context(context);

        output.push_str("Workload typed archive maintenance timing summary\n");
        output.push_str("------------------------------------------------\n");
        output.push_str(
            "workload | action | reason | status write required | base records | pending append | tombstones | resulting records | index before bytes | index after bytes | index byte delta | tombstone before bytes | tombstone after bytes | tombstone byte delta | logical before bytes | logical after bytes | logical byte delta | matched after maintenance | maintenance | agreement\n",
        );

        for workload in &context.workloads {
            let report = typed_archive_maintenance_report(context, &typed_context, workload);

            output.push_str(&format!(
                "{} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | pass\n",
                workload.name,
                format_archive_maintenance_action(report.selected_action),
                format_archive_maintenance_reason(report.selected_reason),
                report.maintenance_status_requires_archive_write,
                report.base_record_count,
                report.pending_append_record_count,
                report.tombstone_count,
                report.resulting_record_count,
                report.query_archive_bytes_before_maintenance,
                report.query_archive_bytes_after_maintenance,
                report.query_archive_byte_delta,
                report.tombstone_archive_bytes_before_maintenance,
                report.tombstone_archive_bytes_after_maintenance,
                report.tombstone_archive_byte_delta,
                report.logical_archive_bytes_before_maintenance,
                report.logical_archive_bytes_after_maintenance,
                report.logical_archive_byte_delta,
                report.matched_records_after_maintenance,
                format_duration_ascii(report.maintenance_timing.average_elapsed),
            ));
        }

        output.push('\n');
    }

    pub(crate) fn append_target_workload_typed_archive_append_delta_maintenance_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        let typed_context = TypedBenchmarkContext::from_benchmark_context(context);

        append_target_workload_debug_section(
            output,
            context,
            "Target workload typed archive append-delta maintenance timing",
            |output, context, workload| {
                let report = typed_archive_append_delta_maintenance_report(
                    context,
                    &typed_context,
                    workload,
                );

                append_target_typed_archive_append_delta_maintenance_report(output, &report);
            },
        );
    }

    pub(crate) fn append_workload_typed_archive_append_delta_maintenance_summary_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        let typed_context = TypedBenchmarkContext::from_benchmark_context(context);

        output.push_str("Workload typed archive append-delta maintenance timing summary\n");
        output.push_str("-------------------------------------------------------------\n");
        output.push_str(
            "workload | action | reason | status write required | base records | pending append | tombstones | resulting records | index before bytes | index after bytes | index byte delta | append before bytes | append after bytes | append byte delta | tombstone before bytes | tombstone after bytes | tombstone byte delta | logical before bytes | logical after bytes | logical byte delta | matched after maintenance | maintenance | agreement\n",
        );

        for workload in &context.workloads {
            let report =
                typed_archive_append_delta_maintenance_report(context, &typed_context, workload);

            output.push_str(&format!(
                "{} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | pass\n",
                workload.name,
                format_archive_maintenance_action(report.selected_action),
                format_archive_maintenance_reason(report.selected_reason),
                report.maintenance_status_requires_archive_write,
                report.base_record_count,
                report.pending_append_record_count,
                report.tombstone_count,
                report.resulting_record_count,
                report.query_archive_bytes_before_maintenance,
                report.query_archive_bytes_after_maintenance,
                report.query_archive_byte_delta,
                report.append_archive_bytes_before_maintenance,
                report.append_archive_bytes_after_maintenance,
                report.append_archive_byte_delta,
                report.tombstone_archive_bytes_before_maintenance,
                report.tombstone_archive_bytes_after_maintenance,
                report.tombstone_archive_byte_delta,
                report.logical_archive_bytes_before_maintenance,
                report.logical_archive_bytes_after_maintenance,
                report.logical_archive_byte_delta,
                report.matched_records_after_maintenance,
                format_duration_ascii(report.maintenance_timing.average_elapsed),
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

fn append_target_typed_archive_append_delta_query_report(
    output: &mut String,
    report: &TypedArchiveAppendDeltaQueryTimingReport,
) {
    append_debug_line(output, "base records", report.base_record_count);
    append_debug_line(output, "appended records", report.appended_record_count);
    append_debug_line(output, "rebuilt records", report.rebuilt_record_count);
    append_debug_line(
        output,
        "append-delta matched records",
        report.append_delta_matched_records,
    );
    append_debug_line(
        output,
        "rebuilt matched records",
        report.rebuilt_matched_records,
    );
    append_debug_line(
        output,
        "append-delta reconstructed records",
        report.append_delta_stats.reconstructed_records,
    );
    append_debug_line(
        output,
        "rebuilt reconstructed records",
        report.rebuilt_stats.reconstructed_records,
    );
    append_debug_line(
        output,
        "append-delta candidate ratio",
        format_scalar_percent(report.append_delta_stats.candidate_ratio),
    );
    append_debug_line(
        output,
        "rebuilt candidate ratio",
        format_scalar_percent(report.rebuilt_stats.candidate_ratio),
    );
    append_debug_duration_line(
        output,
        "rebuilt query average elapsed",
        report.repeated_timing.baseline.average_elapsed,
    );
    append_debug_duration_line(
        output,
        "append-delta query average elapsed",
        report.repeated_timing.fse.average_elapsed,
    );
    append_debug_line(
        output,
        "rebuilt to append-delta ratio",
        format_speedup_ratio(report.rebuilt_to_append_delta_average_ratio),
    );
    append_debug_line(output, "typed archive append-delta query agreement", "pass");
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
        "logical archive bytes before compaction",
        report.logical_archive_bytes_before_compaction,
    );
    append_debug_line(
        output,
        "logical archive bytes after compaction",
        report.logical_archive_bytes_after_compaction,
    );
    append_debug_line(
        output,
        "logical archive byte delta",
        report.logical_archive_byte_delta,
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

fn append_target_typed_archive_maintenance_report(
    output: &mut String,
    report: &TypedArchiveMaintenanceTimingReport,
) {
    append_debug_line(
        output,
        "selected maintenance action",
        format_archive_maintenance_action(report.selected_action),
    );
    append_debug_line(
        output,
        "selected maintenance reason",
        format_archive_maintenance_reason(report.selected_reason),
    );
    append_debug_line(
        output,
        "maintenance status requires archive write",
        report.maintenance_status_requires_archive_write,
    );
    append_debug_line(output, "base records", report.base_record_count);
    append_debug_line(
        output,
        "pending append records",
        report.pending_append_record_count,
    );
    append_debug_line(output, "tombstones", report.tombstone_count);
    append_debug_line(
        output,
        "tombstone ratio basis points",
        report.tombstone_ratio_basis_points,
    );
    append_debug_line(output, "resulting records", report.resulting_record_count);
    append_debug_line(
        output,
        "query archive bytes before maintenance",
        report.query_archive_bytes_before_maintenance,
    );
    append_debug_line(
        output,
        "query archive bytes after maintenance",
        report.query_archive_bytes_after_maintenance,
    );
    append_debug_line(
        output,
        "query archive byte delta",
        report.query_archive_byte_delta,
    );
    append_debug_line(
        output,
        "tombstone archive bytes before maintenance",
        report.tombstone_archive_bytes_before_maintenance,
    );
    append_debug_line(
        output,
        "tombstone archive bytes after maintenance",
        report.tombstone_archive_bytes_after_maintenance,
    );
    append_debug_line(
        output,
        "tombstone archive byte delta",
        report.tombstone_archive_byte_delta,
    );
    append_debug_line(
        output,
        "logical archive bytes before maintenance",
        report.logical_archive_bytes_before_maintenance,
    );
    append_debug_line(
        output,
        "logical archive bytes after maintenance",
        report.logical_archive_bytes_after_maintenance,
    );
    append_debug_line(
        output,
        "logical archive byte delta",
        report.logical_archive_byte_delta,
    );
    append_debug_line(
        output,
        "matched records after maintenance",
        report.matched_records_after_maintenance,
    );
    append_debug_duration_line(
        output,
        "maintenance average elapsed",
        report.maintenance_timing.average_elapsed,
    );
    append_debug_line(output, "typed archive maintenance agreement", "pass");
}

fn append_target_typed_archive_append_delta_maintenance_report(
    output: &mut String,
    report: &TypedArchiveAppendDeltaMaintenanceTimingReport,
) {
    append_debug_line(
        output,
        "selected maintenance action",
        format_archive_maintenance_action(report.selected_action),
    );
    append_debug_line(
        output,
        "selected maintenance reason",
        format_archive_maintenance_reason(report.selected_reason),
    );
    append_debug_line(
        output,
        "maintenance status requires archive write",
        report.maintenance_status_requires_archive_write,
    );
    append_debug_line(output, "base records", report.base_record_count);
    append_debug_line(
        output,
        "pending append records",
        report.pending_append_record_count,
    );
    append_debug_line(output, "tombstones", report.tombstone_count);
    append_debug_line(
        output,
        "tombstone ratio basis points",
        report.tombstone_ratio_basis_points,
    );
    append_debug_line(output, "resulting records", report.resulting_record_count);
    append_debug_line(
        output,
        "query archive bytes before maintenance",
        report.query_archive_bytes_before_maintenance,
    );
    append_debug_line(
        output,
        "query archive bytes after maintenance",
        report.query_archive_bytes_after_maintenance,
    );
    append_debug_line(
        output,
        "query archive byte delta",
        report.query_archive_byte_delta,
    );
    append_debug_line(
        output,
        "append archive bytes before maintenance",
        report.append_archive_bytes_before_maintenance,
    );
    append_debug_line(
        output,
        "append archive bytes after maintenance",
        report.append_archive_bytes_after_maintenance,
    );
    append_debug_line(
        output,
        "append archive byte delta",
        report.append_archive_byte_delta,
    );
    append_debug_line(
        output,
        "tombstone archive bytes before maintenance",
        report.tombstone_archive_bytes_before_maintenance,
    );
    append_debug_line(
        output,
        "tombstone archive bytes after maintenance",
        report.tombstone_archive_bytes_after_maintenance,
    );
    append_debug_line(
        output,
        "tombstone archive byte delta",
        report.tombstone_archive_byte_delta,
    );
    append_debug_line(
        output,
        "logical archive bytes before maintenance",
        report.logical_archive_bytes_before_maintenance,
    );
    append_debug_line(
        output,
        "logical archive bytes after maintenance",
        report.logical_archive_bytes_after_maintenance,
    );
    append_debug_line(
        output,
        "logical archive byte delta",
        report.logical_archive_byte_delta,
    );
    append_debug_line(
        output,
        "matched records after maintenance",
        report.matched_records_after_maintenance,
    );
    append_debug_duration_line(
        output,
        "append-delta maintenance average elapsed",
        report.maintenance_timing.average_elapsed,
    );
    append_debug_line(
        output,
        "typed archive append-delta maintenance agreement",
        "pass",
    );
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

fn typed_archive_append_delta_query_report(
    context: &BenchmarkApplicationContext,
    typed_context: &TypedBenchmarkContext,
    workload: &QueryWorkloadCase,
) -> TypedArchiveAppendDeltaQueryTimingReport {
    let plan = typed_x_range_plan(typed_context, workload);
    let appended = typed_context.append_batch_from_benchmark_context(context);
    let encoder = typed_context.encoder();
    let builder = FSEBuilder::new(context.suite_config.build_config());
    let report = compare_typed_archive_append_delta_query_execution_repeated(
        typed_context.query_index(),
        &appended,
        &encoder,
        &builder,
        &plan,
        &context.timing_config,
    );

    report.expect("typed archive append-delta query timing should execute")
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

fn typed_archive_maintenance_report(
    context: &BenchmarkApplicationContext,
    typed_context: &TypedBenchmarkContext,
    workload: &QueryWorkloadCase,
) -> TypedArchiveMaintenanceTimingReport {
    let plan = typed_x_range_plan(typed_context, workload);
    let query_index_path = typed_archive_temporary_path(workload);
    let tombstone_path = typed_tombstone_archive_temporary_path(workload);
    let appended = typed_context.append_batch_from_benchmark_context(context);
    let tombstone_row_ids = typed_archive_compaction_tombstones(typed_context);
    let encoder = typed_context.encoder();
    let builder = FSEBuilder::new(context.suite_config.build_config());
    let policy = typed_archive_maintenance_policy();
    let report = compare_typed_archive_maintenance_execution_repeated(
        &query_index_path,
        &tombstone_path,
        typed_context.query_index(),
        Some(&appended),
        &tombstone_row_ids,
        &encoder,
        &builder,
        &policy,
        &plan,
        &context.timing_config,
    );

    let _ = fs::remove_file(&query_index_path);
    let _ = fs::remove_file(&tombstone_path);

    report.expect("typed archive maintenance timing should execute")
}

fn typed_archive_append_delta_maintenance_report(
    context: &BenchmarkApplicationContext,
    typed_context: &TypedBenchmarkContext,
    workload: &QueryWorkloadCase,
) -> TypedArchiveAppendDeltaMaintenanceTimingReport {
    let plan = typed_x_range_plan(typed_context, workload);
    let query_index_path = typed_archive_temporary_path(workload);
    let append_path = typed_append_archive_temporary_path(workload);
    let tombstone_path = typed_tombstone_archive_temporary_path(workload);
    let appended = typed_context.append_batch_from_benchmark_context(context);
    let tombstone_row_ids = typed_archive_compaction_tombstones(typed_context);
    let encoder = typed_context.encoder();
    let builder = FSEBuilder::new(context.suite_config.build_config());
    let policy = typed_archive_maintenance_policy();
    let report = compare_typed_archive_append_delta_maintenance_execution_repeated(
        &query_index_path,
        &append_path,
        &tombstone_path,
        typed_context.query_index(),
        &appended,
        &tombstone_row_ids,
        &encoder,
        &builder,
        &policy,
        &plan,
        &context.timing_config,
    );

    let _ = fs::remove_file(&query_index_path);
    let _ = fs::remove_file(&append_path);
    let _ = fs::remove_file(&tombstone_path);

    report.expect("typed archive append-delta maintenance timing should execute")
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

fn typed_archive_maintenance_policy() -> FSEArchiveMaintenancePolicy {
    FSEArchiveMaintenancePolicy::try_new(
        MAINTENANCE_APPEND_REBUILD_RECORD_COUNT_THRESHOLD,
        MAINTENANCE_COMPACTION_TOMBSTONE_COUNT_THRESHOLD,
        MAINTENANCE_COMPACTION_TOMBSTONE_RATIO_THRESHOLD_BASIS_POINTS,
    )
    .expect("typed archive maintenance benchmark policy should stay valid")
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

fn typed_append_archive_temporary_path(workload: &QueryWorkloadCase) -> PathBuf {
    let path_id = ARCHIVE_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
    let file_name = format!(
        "fse-typed-append-archive-{}-{}-{}{}",
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

fn format_archive_maintenance_action(action: FSEArchiveMaintenanceAction) -> &'static str {
    match action {
        FSEArchiveMaintenanceAction::NoMaintenance => "no_maintenance",
        FSEArchiveMaintenanceAction::Append => "append",
        FSEArchiveMaintenanceAction::Compact => "compact",
        FSEArchiveMaintenanceAction::Rebuild => "rebuild",
    }
}

fn format_archive_maintenance_reason(reason: FSEArchiveMaintenanceReason) -> &'static str {
    match reason {
        FSEArchiveMaintenanceReason::NoPendingMaintenance => "no_pending_maintenance",
        FSEArchiveMaintenanceReason::PendingAppendRecords => "pending_append_records",
        FSEArchiveMaintenanceReason::AppendRebuildThresholdReached => {
            "append_rebuild_threshold_reached"
        }
        FSEArchiveMaintenanceReason::CompactionTombstoneCountThresholdReached => {
            "compaction_tombstone_count_threshold_reached"
        }
        FSEArchiveMaintenanceReason::CompactionTombstoneRatioThresholdReached => {
            "compaction_tombstone_ratio_threshold_reached"
        }
        FSEArchiveMaintenanceReason::AppendAndCompactionThresholdsReached => {
            "append_and_compaction_thresholds_reached"
        }
    }
}

fn format_scalar_percent(value: crate::math::Scalar) -> String {
    format_percent_ratio(value as f64)
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
