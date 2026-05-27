//! Target workload stage timing diagnostics.

use super::super::context::BenchmarkApplicationContext;
use super::super::renderer::BenchmarkApplicationRenderer;
use super::formatting::format_percent_ratio;
use super::target::{
    append_debug_duration_line, append_debug_line, append_target_workload_debug_section,
};
use crate::benchmark::reports::{duration_ratio, measure_repeated};
use crate::query::{execute_query_with_stats_and_options, traverse_with_stats};

impl BenchmarkApplicationRenderer {
    pub(crate) fn append_target_workload_stage_timing_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        append_target_workload_debug_section(
            output,
            context,
            "Target workload stage timing estimate",
            |output, context, workload| {
                let timing_config = &context.timing_config;
                let query_options = context.suite_config.query_execution_options();

                // this is an estimate not a benchmark verdict
                let traversal_timing = measure_repeated(timing_config, || {
                    let _ = traverse_with_stats(&context.index, &workload.query);
                });

                let full_fse_timing = measure_repeated(timing_config, || {
                    let _ = execute_query_with_stats_and_options(
                        &context.index,
                        &workload.query,
                        query_options,
                    );
                });

                let traversal = traverse_with_stats(&context.index, &workload.query);
                let full_report = execute_query_with_stats_and_options(
                    &context.index,
                    &workload.query,
                    query_options,
                );
                let estimated_non_traversal_elapsed = full_fse_timing
                    .average_elapsed
                    .saturating_sub(traversal_timing.average_elapsed);
                let traversal_share = duration_ratio(
                    traversal_timing.average_elapsed,
                    full_fse_timing.average_elapsed,
                );

                append_debug_line(output, "timing iterations", timing_config.iterations);
                append_debug_duration_line(
                    output,
                    "average traversal elapsed",
                    traversal_timing.average_elapsed,
                );
                append_debug_duration_line(
                    output,
                    "average full FSE elapsed",
                    full_fse_timing.average_elapsed,
                );
                append_debug_duration_line(
                    output,
                    "estimated non-traversal elapsed",
                    estimated_non_traversal_elapsed,
                );
                append_debug_line(
                    output,
                    "estimated traversal share",
                    format_percent_ratio(traversal_share),
                );
                append_debug_line(output, "retained leaves", traversal.stats.retained_leaves);
                append_debug_line(
                    output,
                    "candidate records",
                    traversal.stats.retained_candidate_records,
                );
                append_debug_line(output, "matched records", full_report.stats.matched_records);
            },
        );
    }
}
