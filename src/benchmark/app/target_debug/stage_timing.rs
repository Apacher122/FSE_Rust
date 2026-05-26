//! Target workload stage timing diagnostics.

use super::super::context::BenchmarkApplicationContext;
use super::super::renderer::BenchmarkApplicationRenderer;
use super::formatting::format_percent_ratio;
use super::target::append_target_workload_debug_section;
use crate::benchmark::reports::output::format_duration_ascii;
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

                output.push_str(&format!(
                    "timing iterations: {}\n",
                    timing_config.iterations
                ));
                output.push_str(&format!(
                    "average traversal elapsed: {}\n",
                    format_duration_ascii(traversal_timing.average_elapsed)
                ));
                output.push_str(&format!(
                    "average full FSE elapsed: {}\n",
                    format_duration_ascii(full_fse_timing.average_elapsed)
                ));
                output.push_str(&format!(
                    "estimated non-traversal elapsed: {}\n",
                    format_duration_ascii(estimated_non_traversal_elapsed)
                ));
                output.push_str(&format!(
                    "estimated traversal share: {}\n",
                    format_percent_ratio(traversal_share)
                ));
                output.push_str(&format!(
                    "retained leaves: {}\n",
                    traversal.stats.retained_leaves
                ));
                output.push_str(&format!(
                    "candidate records: {}\n",
                    traversal.stats.retained_candidate_records
                ));
                output.push_str(&format!(
                    "matched records: {}\n",
                    full_report.stats.matched_records
                ));
            },
        );
    }
}
