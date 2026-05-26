//! Target workload retained reconstruction timing diagnostics.

use super::super::context::BenchmarkApplicationContext;
use super::super::renderer::BenchmarkApplicationRenderer;
use super::candidates::retained_candidate_breakdown;
use super::formatting::format_percent_ratio;
use super::target::append_target_workload_debug_section;
use crate::benchmark::reports::output::format_duration_ascii;
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

                output.push_str(&format!(
                    "timing iterations: {}\n",
                    timing_config.iterations
                ));
                output.push_str(&format!(
                    "average retained execution elapsed: {}\n",
                    format_duration_ascii(retained_execution_timing.average_elapsed)
                ));
                output.push_str(&format!(
                    "average full FSE elapsed: {}\n",
                    format_duration_ascii(full_fse_timing.average_elapsed)
                ));
                output.push_str(&format!(
                    "estimated retained execution share: {}\n",
                    format_percent_ratio(retained_execution_share)
                ));
                output.push_str(&format!(
                    "retained leaves: {}\n",
                    traversal.stats.retained_leaves
                ));
                output.push_str(&format!(
                    "covered leaves: {}\n",
                    retained_breakdown.covered_leaves
                ));
                output.push_str(&format!(
                    "partial leaves: {}\n",
                    retained_breakdown.partial_leaves
                ));
                output.push_str(&format!(
                    "candidate records: {}\n",
                    traversal.stats.retained_candidate_records
                ));
                output.push_str(&format!(
                    "covered records: {}\n",
                    retained_breakdown.covered_records
                ));
                output.push_str(&format!(
                    "partial records: {}\n",
                    retained_breakdown.partial_records
                ));
                output.push_str(&format!(
                    "matched records: {}\n",
                    retained_report.matched_records
                ));
            },
        );
    }
}
