//! Target workload retained execution phase diagnostics.

use super::super::context::BenchmarkApplicationContext;
use super::super::renderer::BenchmarkApplicationRenderer;
use super::candidates::{
    collect_matching_values_as_results, count_matching_retained_candidate_rows,
    matching_retained_candidate_values, reconstruct_retained_candidate_rows,
    retained_candidate_breakdown,
};
use super::target::append_target_workload_debug_section;
use crate::benchmark::reports::measure_repeated;
use crate::benchmark::reports::output::format_duration_ascii;
use crate::query::{execute_retained_leaf_batch_for_diagnostics, traverse_with_stats};

impl BenchmarkApplicationRenderer {
    pub(crate) fn append_target_workload_retained_execution_phase_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        append_target_workload_debug_section(
            output,
            context,
            "Target workload retained execution phase estimate",
            |output, context, workload| {
                let timing_config = &context.timing_config;
                let query_options = context.suite_config.query_execution_options();
                let traversal = traverse_with_stats(&context.index, &workload.query);
                let retained_breakdown =
                    retained_candidate_breakdown(context, &traversal.retained_leaves);

                let reconstructed_rows =
                    reconstruct_retained_candidate_rows(context, &traversal.retained_leaves);
                let matched_values =
                    matching_retained_candidate_values(&workload.query, &reconstructed_rows);

                let reconstruction_timing = measure_repeated(timing_config, || {
                    let rows =
                        reconstruct_retained_candidate_rows(context, &traversal.retained_leaves);
                    std::hint::black_box(rows.len());
                });

                let predicate_timing = measure_repeated(timing_config, || {
                    let matched_count = count_matching_retained_candidate_rows(
                        &workload.query,
                        &reconstructed_rows,
                    );
                    std::hint::black_box(matched_count);
                });

                let result_collection_timing = measure_repeated(timing_config, || {
                    let results = collect_matching_values_as_results(&matched_values);
                    std::hint::black_box(results);
                });

                let retained_execution_timing = measure_repeated(timing_config, || {
                    let _ = execute_retained_leaf_batch_for_diagnostics(
                        &context.index,
                        &workload.query,
                        &traversal.retained_leaves,
                        traversal.stats.retained_candidate_records,
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

                output.push_str(&format!(
                    "timing iterations: {}\n",
                    timing_config.iterations
                ));
                output.push_str(&format!(
                    "average retained reconstruction elapsed: {}\n",
                    format_duration_ascii(reconstruction_timing.average_elapsed)
                ));
                output.push_str(&format!(
                    "average retained predicate elapsed: {}\n",
                    format_duration_ascii(predicate_timing.average_elapsed)
                ));
                output.push_str(&format!(
                    "average retained result collection elapsed: {}\n",
                    format_duration_ascii(result_collection_timing.average_elapsed)
                ));
                output.push_str(&format!(
                    "average retained execution elapsed: {}\n",
                    format_duration_ascii(retained_execution_timing.average_elapsed)
                ));
                output.push_str(&format!(
                    "candidate records: {}\n",
                    traversal.stats.retained_candidate_records
                ));
                output.push_str(&format!(
                    "matched records: {}\n",
                    retained_report.matched_records
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
                    "covered records: {}\n",
                    retained_breakdown.covered_records
                ));
                output.push_str(&format!(
                    "partial records: {}\n",
                    retained_breakdown.partial_records
                ));
            },
        );
    }
}
