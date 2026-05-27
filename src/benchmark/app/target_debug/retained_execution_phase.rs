//! Target workload retained execution phase diagnostics.

use super::super::context::BenchmarkApplicationContext;
use super::super::renderer::BenchmarkApplicationRenderer;
use super::candidates::{
    collect_matching_values_as_results, count_matching_retained_candidate_rows,
    matching_retained_candidate_values, reconstruct_retained_candidate_rows,
    retained_candidate_breakdown,
};
use super::target::{
    append_debug_duration_line, append_debug_line, append_retained_candidate_breakdown,
    append_target_workload_debug_section,
};
use crate::benchmark::reports::measure_repeated;
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

                append_debug_line(output, "timing iterations", timing_config.iterations);
                append_debug_duration_line(
                    output,
                    "average retained reconstruction elapsed",
                    reconstruction_timing.average_elapsed,
                );
                append_debug_duration_line(
                    output,
                    "average retained predicate elapsed",
                    predicate_timing.average_elapsed,
                );
                append_debug_duration_line(
                    output,
                    "average retained result collection elapsed",
                    result_collection_timing.average_elapsed,
                );
                append_debug_duration_line(
                    output,
                    "average retained execution elapsed",
                    retained_execution_timing.average_elapsed,
                );
                append_debug_line(
                    output,
                    "candidate records",
                    traversal.stats.retained_candidate_records,
                );
                append_debug_line(output, "matched records", retained_report.matched_records);
                append_retained_candidate_breakdown(output, &retained_breakdown);
            },
        );
    }
}
