//! Target workload count-only comparison diagnostics.

use super::super::context::BenchmarkApplicationContext;
use super::super::renderer::BenchmarkApplicationRenderer;
use super::formatting::format_speedup_ratio;
use super::target::append_target_workload_debug_section;
use crate::benchmark::reports::output::format_duration_ascii;
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

                output.push_str(&format!(
                    "timing iterations: {}\n",
                    timing_config.iterations
                ));
                output.push_str(&format!(
                    "owned average elapsed: {}\n",
                    format_duration_ascii(owned_timing.average_elapsed)
                ));
                output.push_str(&format!(
                    "count-only average elapsed: {}\n",
                    format_duration_ascii(count_only_timing.average_elapsed)
                ));
                output.push_str(&format!(
                    "estimated owned result overhead: {}\n",
                    format_duration_ascii(estimated_owned_result_overhead)
                ));
                output.push_str(&format!(
                    "count-only speedup: {}\n",
                    format_speedup_ratio(owned_count_speedup)
                ));
                output.push_str(&format!(
                    "owned matched records: {}\n",
                    owned_report.stats.matched_records
                ));
                output.push_str(&format!(
                    "count-only matched records: {}\n",
                    count_report.matched_records
                ));
                output.push_str(&format!(
                    "matched records agree: {}\n",
                    owned_report.stats.matched_records == count_report.matched_records
                ));
                output.push_str(&format!(
                    "owned candidate records: {}\n",
                    owned_report.stats.reconstructed_records
                ));
                output.push_str(&format!(
                    "count-only candidate records: {}\n",
                    count_report.stats.reconstructed_records
                ));
                output.push_str(&format!(
                    "owned retained leaves: {}\n",
                    owned_report.stats.retained_leaves
                ));
                output.push_str(&format!(
                    "count-only retained leaves: {}\n",
                    count_report.stats.retained_leaves
                ));
                output.push_str(&format!(
                    "owned visited nodes: {}\n",
                    owned_report.stats.visited_nodes
                ));
                output.push_str(&format!(
                    "count-only visited nodes: {}\n",
                    count_report.stats.visited_nodes
                ));
            },
        );
    }
}
