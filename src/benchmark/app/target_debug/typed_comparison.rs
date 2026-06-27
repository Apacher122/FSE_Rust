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
use crate::query::TypedQueryPlanningRiskFlags;

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
            "workload | matched | typed scan | indexed typed | timing ratio | candidate ratio | planner strategy | planner reason | planner cost classification | hierarchy metadata bytes | records pruned | selectivity bucket | planner risk | planner predicate delta | planner flat scan delta | planner traversal delta | record avoidance | agreement\n",
        );

        for workload in &context.workloads {
            let report = typed_comparison_report(context, &typed_context, workload);
            let cost_comparison = report
                .planning_diagnostics
                .cost_comparison_against_flat_scan();

            output.push_str(&format!(
                "{} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | pass\n",
                workload.name,
                report.indexed_matched_records,
                format_duration_ascii(report.repeated_timing.baseline.average_elapsed),
                format_duration_ascii(report.repeated_timing.fse.average_elapsed),
                format_speedup_ratio(report.average_timing_ratio),
                format_scalar_percent(report.candidate_ratio),
                format!("{:?}", report.planning_diagnostics.strategy),
                format!("{:?}", report.planning_diagnostics.reason),
                format!(
                    "{:?}",
                    report
                        .planning_diagnostics
                        .cost_classification_against_flat_scan()
                ),
                report.planning_diagnostics.hierarchy_metadata_bytes,
                report.indexed_stats.records_pruned(),
                format!("{:?}", report.planning_diagnostics.selectivity_bucket),
                format_planner_risk_flags(report.planning_diagnostics.risk_flags),
                cost_comparison.predicate_evaluation_delta,
                cost_comparison.flat_scan_record_delta,
                cost_comparison.traversal_node_visit_delta,
                format_scalar_percent(report.record_evaluation_avoidance_ratio),
            ));
        }

        output.push('\n');
    }
}

fn append_target_typed_comparison_report(output: &mut String, report: &TypedQueryComparisonReport) {
    let cost_comparison = report
        .planning_diagnostics
        .cost_comparison_against_flat_scan();

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
    append_debug_line(
        output,
        "planner strategy",
        format!("{:?}", report.planning_diagnostics.strategy),
    );
    append_debug_line(
        output,
        "planner reason",
        format!("{:?}", report.planning_diagnostics.reason),
    );
    append_debug_line(
        output,
        "planner cost classification",
        format!(
            "{:?}",
            report
                .planning_diagnostics
                .cost_classification_against_flat_scan()
        ),
    );
    append_debug_line(
        output,
        "hierarchy metadata bytes",
        report.planning_diagnostics.hierarchy_metadata_bytes,
    );
    append_debug_line(
        output,
        "records pruned",
        report.indexed_stats.records_pruned(),
    );
    append_debug_line(
        output,
        "planner selectivity bucket",
        format!("{:?}", report.planning_diagnostics.selectivity_bucket),
    );
    append_debug_line(
        output,
        "planner risk",
        format_planner_risk_flags(report.planning_diagnostics.risk_flags),
    );
    append_debug_line(
        output,
        "planner estimated traversal node visits",
        report
            .planning_diagnostics
            .work_estimate
            .estimated_traversal_node_visits,
    );
    append_debug_line(
        output,
        "planner estimated reconstructed records",
        report
            .planning_diagnostics
            .work_estimate
            .estimated_reconstructed_records,
    );
    append_debug_line(
        output,
        "planner estimated predicate evaluations",
        report
            .planning_diagnostics
            .work_estimate
            .estimated_predicate_evaluations,
    );
    append_debug_line(
        output,
        "planner estimated flat scan records",
        report
            .planning_diagnostics
            .work_estimate
            .estimated_flat_scan_records,
    );
    append_debug_line(
        output,
        "planner predicate evaluation delta vs flat scan",
        cost_comparison.predicate_evaluation_delta,
    );
    append_debug_line(
        output,
        "planner flat scan record delta vs flat scan",
        cost_comparison.flat_scan_record_delta,
    );
    append_debug_line(
        output,
        "planner traversal node visit delta vs flat scan",
        cost_comparison.traversal_node_visit_delta,
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

fn format_planner_risk_flags(flags: TypedQueryPlanningRiskFlags) -> String {
    let mut risks = Vec::new();

    if flags.broad_predicate {
        risks.push("broad");
    }

    if flags.materialization_pressure {
        risks.push("materialization");
    }

    if flags.high_dimensional_low_constraint {
        risks.push("high_dimensional_low_constraint");
    }

    if flags.append_delta_scan {
        risks.push("append_delta");
    }

    if risks.is_empty() {
        "none".to_string()
    } else {
        risks.join("+")
    }
}
