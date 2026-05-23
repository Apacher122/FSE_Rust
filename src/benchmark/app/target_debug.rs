//! Target workload debug rendering for benchmark application output.
//!
//! This module keeps boundary-workload diagnostics separate from the general
//! benchmark terminal renderer. The diagnostics are intentionally debug-only and
//! should not change query execution behavior.

use super::context::BenchmarkApplicationContext;
use super::renderer::BenchmarkApplicationRenderer;
use crate::benchmark::BenchmarkDatasetKind;
use crate::benchmark::reports::output::format_duration_ascii;
use crate::benchmark::reports::{duration_ratio, measure_repeated};
use crate::math::{BoundingBox, Scalar, Vector};
use crate::query::{
    QueryRegion, RetainedLeaf, RetainedLeafCoverage, execute_query_with_stats_and_options,
    execute_retained_leaf_batch_for_diagnostics, traverse_with_stats,
};
use crate::storage::{LeafReconstructionShape, PartitionNode};

impl BenchmarkApplicationRenderer {
    pub(crate) fn append_target_workload_retained_leaf_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        output.push_str("Target workload retained leaf details\n");
        output.push_str("-------------------------------------\n");

        let target_workload_name = target_boundary_workload_name(context);

        let Some(workload) = context
            .workloads
            .iter()
            .find(|workload| workload.name == target_workload_name)
        else {
            output.push_str(&format!("workload: {}\n", target_workload_name));
            output.push_str("status: workload not found\n\n");
            return;
        };

        let traversal = traverse_with_stats(&context.index, &workload.query);

        output.push_str(&format!("workload: {}\n", workload.name));
        output.push_str(&format!(
            "query min: {}\n",
            format_coordinate_values(&workload.query.min)
        ));
        output.push_str(&format!(
            "query max: {}\n",
            format_coordinate_values(&workload.query.max)
        ));
        output.push_str(&format!(
            "retained leaves: {}\n",
            traversal.stats.retained_leaves
        ));
        output.push_str("leaf | coverage | records | bounds min | bounds max | volume\n");

        if traversal.retained_leaves.is_empty() {
            output.push_str("none\n");
            output.push('\n');
            return;
        }

        for retained_leaf in &traversal.retained_leaves {
            let node = &context.index.nodes[retained_leaf.node_id];

            output.push_str(&format!(
                "{} | {} | {} | {} | {} | {:.2}\n",
                retained_leaf.node_id,
                retained_leaf_coverage_label(retained_leaf.coverage),
                node.stored_cardinality(),
                format_bounds_min(&node.bounds),
                format_bounds_max(&node.bounds),
                node.bounds.volume(),
            ));
        }

        output.push('\n');
    }

    pub(crate) fn append_target_workload_stage_timing_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        output.push_str("Target workload stage timing estimate\n");
        output.push_str("-------------------------------------\n");

        let target_workload_name = target_boundary_workload_name(context);

        let Some(workload) = context
            .workloads
            .iter()
            .find(|workload| workload.name == target_workload_name)
        else {
            output.push_str(&format!("workload: {}\n", target_workload_name));
            output.push_str("status: workload not found\n\n");
            return;
        };

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
        let full_report =
            execute_query_with_stats_and_options(&context.index, &workload.query, query_options);
        let estimated_non_traversal_elapsed = full_fse_timing
            .average_elapsed
            .saturating_sub(traversal_timing.average_elapsed);
        let traversal_share = duration_ratio(
            traversal_timing.average_elapsed,
            full_fse_timing.average_elapsed,
        );

        output.push_str(&format!("workload: {}\n", workload.name));
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
        output.push('\n');
    }

    pub(crate) fn append_target_workload_reconstruction_timing_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        output.push_str("Target workload reconstruction timing estimate\n");
        output.push_str("----------------------------------------------\n");

        let target_workload_name = target_boundary_workload_name(context);

        let Some(workload) = context
            .workloads
            .iter()
            .find(|workload| workload.name == target_workload_name)
        else {
            output.push_str(&format!("workload: {}\n", target_workload_name));
            output.push_str("status: workload not found\n\n");
            return;
        };

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
        let retained_breakdown = retained_candidate_breakdown(context, &traversal.retained_leaves);
        let retained_execution_share = duration_ratio(
            retained_execution_timing.average_elapsed,
            full_fse_timing.average_elapsed,
        );

        output.push_str(&format!("workload: {}\n", workload.name));
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
        output.push('\n');
    }

    pub(crate) fn append_target_workload_retained_execution_phase_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        output.push_str("Target workload retained execution phase estimate\n");
        output.push_str("-------------------------------------------------\n");

        let target_workload_name = target_boundary_workload_name(context);

        let Some(workload) = context
            .workloads
            .iter()
            .find(|workload| workload.name == target_workload_name)
        else {
            output.push_str(&format!("workload: {}\n", target_workload_name));
            output.push_str("status: workload not found\n\n");
            return;
        };

        let timing_config = &context.timing_config;
        let query_options = context.suite_config.query_execution_options();
        let traversal = traverse_with_stats(&context.index, &workload.query);
        let retained_breakdown = retained_candidate_breakdown(context, &traversal.retained_leaves);

        let reconstructed_rows =
            reconstruct_retained_candidate_rows(context, &traversal.retained_leaves);
        let matched_values =
            matching_retained_candidate_values(&workload.query, &reconstructed_rows);

        let reconstruction_timing = measure_repeated(timing_config, || {
            let rows = reconstruct_retained_candidate_rows(context, &traversal.retained_leaves);
            std::hint::black_box(rows.len());
        });

        let predicate_timing = measure_repeated(timing_config, || {
            let matched_count =
                count_matching_retained_candidate_rows(&workload.query, &reconstructed_rows);
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

        output.push_str(&format!("workload: {}\n", workload.name));
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
        output.push('\n');
    }

    pub(crate) fn append_target_workload_retained_allocation_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        output.push_str("Target workload retained allocation estimate\n");
        output.push_str("--------------------------------------------\n");

        let target_workload_name = target_boundary_workload_name(context);

        let Some(workload) = context
            .workloads
            .iter()
            .find(|workload| workload.name == target_workload_name)
        else {
            output.push_str(&format!("workload: {}\n", target_workload_name));
            output.push_str("status: workload not found\n\n");
            return;
        };

        let timing_config = &context.timing_config;
        let query_options = context.suite_config.query_execution_options();
        let traversal = traverse_with_stats(&context.index, &workload.query);
        let reconstructed_rows =
            reconstruct_retained_candidate_rows(context, &traversal.retained_leaves);
        let matched_values =
            matching_retained_candidate_values(&workload.query, &reconstructed_rows);
        let retained_breakdown = retained_candidate_breakdown(context, &traversal.retained_leaves);

        let empty_result_allocation_timing = measure_repeated(timing_config, || {
            let results: Vec<Vector> = Vec::new();
            let _ = std::hint::black_box(results);
        });

        let matched_result_allocation_timing = measure_repeated(timing_config, || {
            let results: Vec<Vector> = Vec::with_capacity(matched_values.len());
            let _ = std::hint::black_box(results);
        });

        let candidate_result_allocation_timing = measure_repeated(timing_config, || {
            let results: Vec<Vector> =
                Vec::with_capacity(traversal.stats.retained_candidate_records);
            let _ = std::hint::black_box(results);
        });

        let vector_clone_collection_timing = measure_repeated(timing_config, || {
            let results = collect_matching_values_as_results(&matched_values);
            let _ = std::hint::black_box(results);
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

        output.push_str(&format!("workload: {}\n", workload.name));
        output.push_str(&format!(
            "timing iterations: {}\n",
            timing_config.iterations
        ));
        output.push_str(&format!(
            "candidate records: {}\n",
            traversal.stats.retained_candidate_records
        ));
        output.push_str(&format!("matched records: {}\n", matched_values.len()));
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
        output.push_str(&format!(
            "average empty result allocation elapsed: {}\n",
            format_duration_ascii(empty_result_allocation_timing.average_elapsed)
        ));
        output.push_str(&format!(
            "average matched result allocation elapsed: {}\n",
            format_duration_ascii(matched_result_allocation_timing.average_elapsed)
        ));
        output.push_str(&format!(
            "average candidate result allocation elapsed: {}\n",
            format_duration_ascii(candidate_result_allocation_timing.average_elapsed)
        ));
        output.push_str(&format!(
            "average vector clone collection elapsed: {}\n",
            format_duration_ascii(vector_clone_collection_timing.average_elapsed)
        ));
        output.push_str(&format!(
            "average retained execution elapsed: {}\n",
            format_duration_ascii(retained_execution_timing.average_elapsed)
        ));
        output.push('\n');
    }
}

const SMALL_TARGET_BOUNDARY_WORKLOAD_NAME: &str = "cluster_boundary_range";
const LARGE_TARGET_BOUNDARY_WORKLOAD_NAME: &str = "large_cross_cluster_boundary";

fn target_boundary_workload_name(context: &BenchmarkApplicationContext) -> &'static str {
    match context.suite_config.dataset_kind {
        BenchmarkDatasetKind::SmallClustered2D => SMALL_TARGET_BOUNDARY_WORKLOAD_NAME,
        BenchmarkDatasetKind::LargeClustered2D => LARGE_TARGET_BOUNDARY_WORKLOAD_NAME,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RetainedCandidateBreakdown {
    covered_leaves: usize,
    partial_leaves: usize,
    covered_records: usize,
    partial_records: usize,
}

#[derive(Clone, Debug, PartialEq)]
struct RetainedCandidateRow {
    values: Vec<Scalar>,
    coverage: RetainedLeafCoverage,
}

fn retained_candidate_breakdown(
    context: &BenchmarkApplicationContext,
    retained_leaves: &[RetainedLeaf],
) -> RetainedCandidateBreakdown {
    let mut breakdown = RetainedCandidateBreakdown::default();

    for retained_leaf in retained_leaves {
        let records = context.index.nodes[retained_leaf.node_id].stored_cardinality();

        match retained_leaf.coverage {
            RetainedLeafCoverage::Covered => {
                breakdown.covered_leaves += 1;
                breakdown.covered_records += records;
            }
            RetainedLeafCoverage::Partial => {
                breakdown.partial_leaves += 1;
                breakdown.partial_records += records;
            }
        }
    }

    breakdown
}

fn reconstruct_retained_candidate_rows(
    context: &BenchmarkApplicationContext,
    retained_leaves: &[RetainedLeaf],
) -> Vec<RetainedCandidateRow> {
    let candidate_count = retained_leaves
        .iter()
        .map(|retained_leaf| context.index.nodes[retained_leaf.node_id].stored_cardinality())
        .sum();

    let mut rows = Vec::with_capacity(candidate_count);

    for retained_leaf in retained_leaves {
        let node = &context.index.nodes[retained_leaf.node_id];
        let shape = retained_leaf.reconstruction_shape(&context.index);

        append_reconstructed_candidate_rows(node, shape, retained_leaf.coverage, &mut rows);
    }

    rows
}

fn append_reconstructed_candidate_rows(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    coverage: RetainedLeafCoverage,
    rows: &mut Vec<RetainedCandidateRow>,
) {
    match shape.dimensions {
        1 => append_reconstructed_1d_candidate_rows(node, shape, coverage, rows),
        2 => append_reconstructed_2d_candidate_rows(node, shape, coverage, rows),
        _ => append_reconstructed_generic_candidate_rows(node, shape, coverage, rows),
    }
}

fn append_reconstructed_1d_candidate_rows(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    coverage: RetainedLeafCoverage,
    rows: &mut Vec<RetainedCandidateRow>,
) {
    let centroid_0 = node.centroid[0];
    let residual_0 = &node.residuals.dimensions[0];

    for row in 0..shape.cardinality {
        rows.push(RetainedCandidateRow {
            values: vec![centroid_0 + residual_0[row]],
            coverage,
        });
    }
}

fn append_reconstructed_2d_candidate_rows(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    coverage: RetainedLeafCoverage,
    rows: &mut Vec<RetainedCandidateRow>,
) {
    let centroid_0 = node.centroid[0];
    let centroid_1 = node.centroid[1];

    let residual_0 = &node.residuals.dimensions[0];
    let residual_1 = &node.residuals.dimensions[1];

    // this is diagnostic-only and intentionally mirrors the 2d retained path
    for row in 0..shape.cardinality {
        rows.push(RetainedCandidateRow {
            values: vec![centroid_0 + residual_0[row], centroid_1 + residual_1[row]],
            coverage,
        });
    }
}

fn append_reconstructed_generic_candidate_rows(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    coverage: RetainedLeafCoverage,
    rows: &mut Vec<RetainedCandidateRow>,
) {
    for row in 0..shape.cardinality {
        let mut values = Vec::with_capacity(shape.dimensions);

        for (centroid_value, residual_dimension) in
            node.centroid.iter().zip(&node.residuals.dimensions)
        {
            values.push(*centroid_value + residual_dimension[row]);
        }

        rows.push(RetainedCandidateRow { values, coverage });
    }
}

fn count_matching_retained_candidate_rows(
    query: &QueryRegion,
    rows: &[RetainedCandidateRow],
) -> usize {
    rows.iter()
        .filter(|row| retained_candidate_row_matches(query, row))
        .count()
}

fn matching_retained_candidate_values(
    query: &QueryRegion,
    rows: &[RetainedCandidateRow],
) -> Vec<Vec<Scalar>> {
    rows.iter()
        .filter(|row| retained_candidate_row_matches(query, row))
        .map(|row| row.values.clone())
        .collect()
}

fn retained_candidate_row_matches(query: &QueryRegion, row: &RetainedCandidateRow) -> bool {
    match row.coverage {
        RetainedLeafCoverage::Covered => true,
        RetainedLeafCoverage::Partial => {
            query.contains_values_prevalidated(&row.values, row.values.len())
        }
    }
}

fn collect_matching_values_as_results(matched_values: &[Vec<Scalar>]) -> Vec<Vector> {
    let mut results = Vec::with_capacity(matched_values.len());

    for values in matched_values {
        results.push(Vector::new(values.clone()));
    }

    results
}

fn format_percent_ratio(value: f64) -> String {
    if value.is_infinite() {
        return "inf".to_string();
    }

    format!("{:.2}%", value * 100.0)
}

fn retained_leaf_coverage_label(coverage: RetainedLeafCoverage) -> &'static str {
    match coverage {
        RetainedLeafCoverage::Covered => "covered",
        RetainedLeafCoverage::Partial => "partial",
    }
}

fn format_bounds_min(bounds: &BoundingBox) -> String {
    format_coordinate_values(&bounds.min)
}

fn format_bounds_max(bounds: &BoundingBox) -> String {
    format_coordinate_values(&bounds.max)
}

fn format_coordinate_values(values: &[Scalar]) -> String {
    let formatted_values: Vec<String> =
        values.iter().map(|value| format!("{:.2}", value)).collect();

    format!("[{}]", formatted_values.join(", "))
}
