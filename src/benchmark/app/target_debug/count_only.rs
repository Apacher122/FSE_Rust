//! Target workload count-only comparison diagnostics.

use super::super::context::BenchmarkApplicationContext;
use super::super::renderer::BenchmarkApplicationRenderer;
use super::formatting::format_speedup_ratio;
use super::target::{
    append_debug_duration_line, append_debug_line, append_target_workload_debug_section,
};
use crate::benchmark::reports::{duration_ratio, measure_repeated};
use crate::query::{count_query_matches_with_stats, execute_query_with_stats_and_options};

impl BenchmarkApplicationRenderer {
    pub(crate) fn append_target_workload_count_only_comparison_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        append_target_workload_debug_section(
            output,
            context,
            "Target workload count-only comparison",
            |output, context, workload| {
                let timing_config = &context.timing_config;
                let query_options = context.suite_config.query_execution_options();

                let owned_timing = measure_repeated(timing_config, || {
                    let report = execute_query_with_stats_and_options(
                        &context.index,
                        &workload.query,
                        query_options,
                    );

                    std::hint::black_box(report.results.len());
                });

                let count_only_timing = measure_repeated(timing_config, || {
                    let report = count_query_matches_with_stats(&context.index, &workload.query);

                    std::hint::black_box(report.matched_records);
                });

                let owned_report = execute_query_with_stats_and_options(
                    &context.index,
                    &workload.query,
                    query_options,
                );
                let count_report = count_query_matches_with_stats(&context.index, &workload.query);

                let owned_count_speedup = duration_ratio(
                    owned_timing.average_elapsed,
                    count_only_timing.average_elapsed,
                );
                let estimated_owned_result_overhead = owned_timing
                    .average_elapsed
                    .saturating_sub(count_only_timing.average_elapsed);

                append_debug_line(output, "timing iterations", timing_config.iterations);
                append_debug_duration_line(
                    output,
                    "owned average elapsed",
                    owned_timing.average_elapsed,
                );
                append_debug_duration_line(
                    output,
                    "count-only average elapsed",
                    count_only_timing.average_elapsed,
                );
                append_debug_duration_line(
                    output,
                    "estimated owned result overhead",
                    estimated_owned_result_overhead,
                );
                append_debug_line(
                    output,
                    "count-only speedup",
                    format_speedup_ratio(owned_count_speedup),
                );
                append_debug_line(
                    output,
                    "owned matched records",
                    owned_report.stats.matched_records,
                );
                append_debug_line(
                    output,
                    "count-only matched records",
                    count_report.matched_records,
                );
                append_debug_line(
                    output,
                    "matched records agree",
                    owned_report.stats.matched_records == count_report.matched_records,
                );
                append_debug_line(
                    output,
                    "owned candidate records",
                    owned_report.stats.reconstructed_records,
                );
                append_debug_line(
                    output,
                    "count-only candidate records",
                    count_report.stats.reconstructed_records,
                );
                append_debug_line(
                    output,
                    "owned retained leaves",
                    owned_report.stats.retained_leaves,
                );
                append_debug_line(
                    output,
                    "count-only retained leaves",
                    count_report.stats.retained_leaves,
                );
                append_debug_line(
                    output,
                    "owned visited nodes",
                    owned_report.stats.visited_nodes,
                );
                append_debug_line(
                    output,
                    "count-only visited nodes",
                    count_report.stats.visited_nodes,
                );
            },
        );
    }
}
