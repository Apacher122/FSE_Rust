//! Typed indexed query diagnostics.

use super::super::context::BenchmarkApplicationContext;
use super::super::renderer::BenchmarkApplicationRenderer;
use super::formatting::{format_percent_ratio, format_speedup_ratio};
use super::target::{
    append_debug_duration_line, append_debug_line, append_target_workload_debug_section,
};
use super::typed_workload::{TypedBenchmarkContext, typed_x_range_plan};
use crate::benchmark::reports::output::format_duration_ascii;
use crate::benchmark::reports::{
    TypedQueryComparisonReport, compare_typed_query_execution_repeated,
};
use crate::benchmark::workloads::QueryWorkloadCase;

impl BenchmarkApplicationRenderer {
    pub(crate) fn append_target_workload_typed_indexed_comparison_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        let typed_context = TypedBenchmarkContext::from_benchmark_context(context);

        append_target_workload_debug_section(
            output,
            context,
            "Target workload typed indexed comparison",
            |output, context, workload| {
                let report = typed_comparison_report(context, &typed_context, workload);

                append_target_typed_comparison_report(output, &report);
            },
        );
    }

    pub(crate) fn append_workload_typed_indexed_comparison_summary_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        let typed_context = TypedBenchmarkContext::from_benchmark_context(context);

        output.push_str("Workload typed indexed comparison summary\n");
        output.push_str("-----------------------------------------\n");
        output.push_str(
            "workload | matched | typed scan | indexed typed | timing ratio | candidate ratio | record avoidance | agreement\n",
        );

        for workload in &context.workloads {
            let report = typed_comparison_report(context, &typed_context, workload);

            output.push_str(&format!(
                "{} | {} | {} | {} | {} | {} | {} | pass\n",
                workload.name,
                report.indexed_matched_records,
                format_duration_ascii(report.repeated_timing.baseline.average_elapsed),
                format_duration_ascii(report.repeated_timing.fse.average_elapsed),
                format_speedup_ratio(report.average_timing_ratio),
                format_scalar_percent(report.candidate_ratio),
                format_scalar_percent(report.record_evaluation_avoidance_ratio),
            ));
        }

        output.push('\n');
    }
}

fn append_target_typed_comparison_report(output: &mut String, report: &TypedQueryComparisonReport) {
    append_debug_line(
        output,
        "baseline matched records",
        report.baseline_matched_records,
    );
    append_debug_line(
        output,
        "indexed matched records",
        report.indexed_matched_records,
    );
    append_debug_duration_line(
        output,
        "typed scan average elapsed",
        report.repeated_timing.baseline.average_elapsed,
    );
    append_debug_duration_line(
        output,
        "indexed typed average elapsed",
        report.repeated_timing.fse.average_elapsed,
    );
    append_debug_line(
        output,
        "indexed typed speedup",
        format_speedup_ratio(report.average_timing_ratio),
    );
    append_debug_line(
        output,
        "candidate ratio",
        format_scalar_percent(report.candidate_ratio),
    );
    append_debug_line(
        output,
        "retained leaf ratio",
        format_scalar_percent(report.retained_leaf_ratio),
    );
    append_debug_line(
        output,
        "avoided record evaluations",
        report.avoided_record_evaluations,
    );
    append_debug_line(
        output,
        "record evaluation avoidance ratio",
        format_scalar_percent(report.record_evaluation_avoidance_ratio),
    );
    append_debug_line(output, "typed comparison agreement", "pass");
}

fn typed_comparison_report(
    context: &BenchmarkApplicationContext,
    typed_context: &TypedBenchmarkContext,
    workload: &QueryWorkloadCase,
) -> TypedQueryComparisonReport {
    let plan = typed_x_range_plan(typed_context, workload);

    compare_typed_query_execution_repeated(
        typed_context.query_index(),
        &plan,
        &context.timing_config,
    )
    .expect("typed benchmark comparison should execute")
}

fn format_scalar_percent(value: crate::math::Scalar) -> String {
    format_percent_ratio(value as f64)
}
