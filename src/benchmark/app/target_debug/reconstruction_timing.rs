//! Target workload retained reconstruction timing diagnostics.

use super::super::context::BenchmarkApplicationContext;
use super::super::renderer::BenchmarkApplicationRenderer;
use super::candidates::retained_candidate_breakdown;
use super::formatting::format_percent_ratio;
use super::target::{
    append_debug_duration_line, append_debug_line, append_retained_candidate_breakdown,
    append_target_workload_debug_section,
};
use crate::benchmark::reports::{duration_ratio, measure_repeated};
use crate::query::{
    execute_query_with_stats_and_options, execute_retained_leaf_batch_for_diagnostics,
    traverse_with_stats,
};

impl BenchmarkApplicationRenderer {
    pub(crate) fn append_target_workload_reconstruction_timing_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        append_target_workload_debug_section(
            output,
            context,
            "Target workload reconstruction timing estimate",
            |output, context, workload| {
                let timing_config = &context.timing_config;
                let query_options = context.suite_config.query_execution_options();
                let traversal = traverse_with_stats(&context.index, &workload.query);

                // this intentionally measures retained execution after traversal has already run
                let retained_execution_timing = measure_repeated(timing_config, || {
                    let _ = execute_retained_leaf_batch_for_diagnostics(
                        &context.index,
                        &workload.query,
                        &traversal.retained_leaves,
                        traversal.stats.retained_candidate_records,
                        query_options,
                    );
                });

                let full_fse_timing = measure_repeated(timing_config, || {
                    let _ = execute_query_with_stats_and_options(
                        &context.index,
                        &workload.query,
                        query_options,
                    );
                });

                let retained_report = execute_retained_leaf_batch_for_diagnostics(
                    &context.index,
                    &workload.query,
                    &traversal.retained_leaves,
                    traversal.stats.retained_candidate_records,
                    query_options,
                );
                let retained_breakdown =
                    retained_candidate_breakdown(context, &traversal.retained_leaves);
                let retained_execution_share = duration_ratio(
                    retained_execution_timing.average_elapsed,
                    full_fse_timing.average_elapsed,
                );

                append_debug_line(output, "timing iterations", timing_config.iterations);
                append_debug_duration_line(
                    output,
                    "average retained execution elapsed",
                    retained_execution_timing.average_elapsed,
                );
                append_debug_duration_line(
                    output,
                    "average full FSE elapsed",
                    full_fse_timing.average_elapsed,
                );
                append_debug_line(
                    output,
                    "estimated retained execution share",
                    format_percent_ratio(retained_execution_share),
                );
                append_debug_line(output, "retained leaves", traversal.stats.retained_leaves);
                append_retained_candidate_breakdown(output, &retained_breakdown);
                append_debug_line(
                    output,
                    "candidate records",
                    traversal.stats.retained_candidate_records,
                );
                append_debug_line(output, "matched records", retained_report.matched_records);
            },
        );
    }
}
