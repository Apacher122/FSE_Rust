//! Terminal rendering for benchmark application output.

use super::context::BenchmarkApplicationContext;
use super::result_bundle::BenchmarkApplicationResultBundle;
use crate::benchmark::reports::output::format_duration_ascii;
use crate::benchmark::reports::{
    BaselineAggregateSummary, BenchmarkRunOverview, MultiBaselineAggregateSummary,
    SelectivityBucket, render_benchmark_overview, render_multi_baseline_summary,
    render_named_baseline_suite_report, render_selectivity_bucketed_workload_summary,
    render_suite_report, summarize_workloads_by_selectivity,
};
use crate::build::{index_validation_diagnostics, sibling_overlap_metrics};
use crate::math::{BoundingBox, Scalar};
use crate::query::{QueryRegion, RetainedLeafCoverage, reconstruct_row_into, traverse_with_stats};
use crate::storage::PartitionNode;

/// Terminal renderer for benchmark application output.
///
/// # Runtime Role
///
/// `BenchmarkApplicationRenderer` owns terminal rendering for completed
/// benchmark application results. It keeps formatting decisions separate from
/// benchmark setup, execution, and CSV output.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BenchmarkApplicationRenderer;

impl BenchmarkApplicationRenderer {
    /// Creates a benchmark application renderer.
    pub fn new() -> Self {
        Self
    }

    /// Renders terminal output for a completed benchmark application run.
    pub fn render_terminal_output(
        &self,
        context: &BenchmarkApplicationContext,
        result_bundle: &BenchmarkApplicationResultBundle,
    ) -> String {
        if context.uses_debug_report() {
            return self.render_debug_terminal_output(context, result_bundle);
        }

        self.render_summary_terminal_output(
            context,
            &result_bundle.overview,
            &result_bundle.aggregate_summary,
        )
    }

    fn render_summary_terminal_output(
        &self,
        context: &BenchmarkApplicationContext,
        overview: &BenchmarkRunOverview,
        aggregate_summary: &MultiBaselineAggregateSummary,
    ) -> String {
        let mut output = String::new();

        output.push_str("FSE Benchmark Summary\n");
        output.push_str("=====================\n\n");
        output.push_str(&format!("Records: {}\n", overview.dataset_records));
        output.push_str(&format!("Workloads: {}\n", overview.workloads));
        output.push_str(&format!("Baselines: {}\n", overview.baselines));
        output.push_str(&format!(
            "Timing iterations: {}\n",
            overview.timing_iterations
        ));
        output.push_str(&format!("Index nodes: {}\n", overview.index_nodes));
        output.push_str(&format!(
            "Leaves: {} leaf, {} internal\n",
            overview.index_structure.leaf_count, overview.index_structure.internal_node_count
        ));
        output.push_str(&format!(
            "Leaf policy: target {}, max {}\n",
            overview.target_leaf_size, overview.max_leaf_size
        ));
        output.push_str(&format!(
            "Leaf records: max {}, avg {:.2}\n",
            overview.index_structure.max_leaf_cardinality,
            overview.index_structure.average_leaf_cardinality
        ));
        output.push_str(&format!(
            "Leaf volume: total {:.2}, density {:.2}\n",
            overview.index_structure.total_leaf_volume, overview.index_structure.index_density
        ));
        output.push_str(&format!(
            "FSE execution: {}\n",
            overview.fse_execution_mode_name()
        ));
        output.push_str(&format!(
            "FSE parallel min leaves: {}\n",
            overview.fse_parallel_min_retained_leaves
        ));
        output.push_str(&format!(
            "Validation: {}\n\n",
            if overview.validation.is_valid() {
                "pass"
            } else {
                "fail"
            }
        ));

        self.append_index_validation_failure_details(&mut output, context);

        output.push_str("Result\n");
        output.push_str("------\n");
        output.push_str(
            "baseline | timing ratio | result | baseline work | fse work | candidate ratio\n",
        );

        for baseline_summary in &aggregate_summary.baseline_summaries {
            output.push_str(&render_scoreboard_row(baseline_summary));
        }

        output.push('\n');
        output.push_str(&render_best_relative_result(aggregate_summary));
        output.push('\n');
        output.push_str(&render_scoreboard_diagnosis(aggregate_summary));
        output.push('\n');
        output.push_str("Next target:\n");
        output.push_str("  ");
        output.push_str(next_target_message(overview));
        output.push('\n');

        output
    }

    fn render_debug_terminal_output(
        &self,
        context: &BenchmarkApplicationContext,
        result_bundle: &BenchmarkApplicationResultBundle,
    ) -> String {
        let mut output = String::new();

        output.push_str(&render_benchmark_overview(&result_bundle.overview));
        self.append_index_validation_failure_details(&mut output, context);
        self.append_sibling_overlap_debug_output(&mut output, context);
        self.append_traversal_pressure_debug_output(&mut output, result_bundle);
        self.append_low_selectivity_tree_gap_debug_output(&mut output, result_bundle);
        self.append_weakest_low_selectivity_workload_debug_output(&mut output, result_bundle);
        self.append_boundary_workload_pressure_debug_output(&mut output, result_bundle);
        // keep this as debug-only until the boundary data says what to optimize
        self.append_target_workload_retained_leaf_debug_output(&mut output, context);
        self.append_target_workload_retained_record_debug_output(&mut output, context);
        self.append_debug_suite_terminal_output(&mut output, context, result_bundle);

        output
    }

    fn append_index_validation_failure_details(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        if context.validation.is_valid() {
            return;
        }

        let diagnostics =
            index_validation_diagnostics(&context.index, context.suite_config.max_leaf_size);

        output.push_str("Index validation failure details\n");
        output.push_str("--------------------------------\n");
        output.push_str(&format!(
            "leaf cardinality valid: {}\n",
            context.validation.leaf_cardinality_valid
        ));
        output.push_str(&format!(
            "hierarchy topology valid: {}\n",
            context.validation.hierarchy_topology_valid
        ));
        output.push_str(&format!(
            "parent-child bounds valid: {}\n",
            context.validation.parent_child_bounds_valid
        ));
        output.push_str(&format!(
            "leaf cardinality violations: {}\n",
            diagnostics.leaf_cardinality_violations.len()
        ));
        output.push_str(&format!(
            "parent-child bounds violations: {}\n",
            diagnostics.parent_child_bounds_violations.len()
        ));
        output.push_str(&format!(
            "invalid child references: {}\n",
            diagnostics
                .hierarchy_topology
                .invalid_child_references
                .len()
        ));
        output.push_str(&format!(
            "self references: {}\n",
            diagnostics.hierarchy_topology.self_reference_count
        ));
        output.push_str(&format!(
            "leaf nodes with children: {}\n",
            diagnostics
                .hierarchy_topology
                .leaf_nodes_with_children_count
        ));
        output.push_str(&format!(
            "internal nodes without children: {}\n",
            diagnostics
                .hierarchy_topology
                .internal_nodes_without_children_count
        ));
        output.push_str(&format!(
            "unreachable nodes: {}\n",
            diagnostics.hierarchy_topology.unreachable_node_count
        ));

        if let Some(worst_leaf) = diagnostics
            .leaf_cardinality_violations
            .iter()
            .max_by_key(|violation| violation.cardinality)
        {
            output.push_str(&format!(
                "worst leaf: {} has {} records, max {}, overflow {}\n",
                worst_leaf.node_id,
                worst_leaf.cardinality,
                worst_leaf.max_leaf_size,
                worst_leaf.overflow_by
            ));
        }

        if !diagnostics.leaf_cardinality_violations.is_empty() {
            output.push_str("leaf | records | max | overflow | bounds min | bounds max\n");

            for violation in diagnostics.leaf_cardinality_violations.iter().take(10) {
                let node = &context.index.nodes[violation.node_id];

                output.push_str(&format!(
                    "{} | {} | {} | {} | {} | {}\n",
                    violation.node_id,
                    violation.cardinality,
                    violation.max_leaf_size,
                    violation.overflow_by,
                    format_bounds_min(&node.bounds),
                    format_bounds_max(&node.bounds),
                ));
            }

            if diagnostics.leaf_cardinality_violations.len() > 10 {
                output.push_str(&format!(
                    "... {} additional leaf cardinality violations omitted\n",
                    diagnostics.leaf_cardinality_violations.len() - 10
                ));
            }
        }

        if !diagnostics.parent_child_bounds_violations.is_empty() {
            output.push_str("parent | child | parent bounds min | parent bounds max | child bounds min | child bounds max\n");

            for violation in diagnostics.parent_child_bounds_violations.iter().take(10) {
                let parent = &context.index.nodes[violation.parent_id];
                let child = &context.index.nodes[violation.child_id];

                output.push_str(&format!(
                    "{} | {} | {} | {} | {} | {}\n",
                    violation.parent_id,
                    violation.child_id,
                    format_bounds_min(&parent.bounds),
                    format_bounds_max(&parent.bounds),
                    format_bounds_min(&child.bounds),
                    format_bounds_max(&child.bounds),
                ));
            }

            if diagnostics.parent_child_bounds_violations.len() > 10 {
                output.push_str(&format!(
                    "... {} additional parent-child bounds violations omitted\n",
                    diagnostics.parent_child_bounds_violations.len() - 10
                ));
            }
        }

        output.push('\n');
    }

    fn append_sibling_overlap_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        let metrics = sibling_overlap_metrics(&context.index);

        output.push_str("Sibling overlap metrics\n");
        output.push_str("-----------------------\n");
        output.push_str(&format!("sibling pairs: {}\n", metrics.sibling_pair_count));
        output.push_str(&format!(
            "overlapping sibling pairs: {}\n",
            metrics.overlapping_sibling_pair_count
        ));
        output.push_str(&format!(
            "total overlap extent: {:.2}\n",
            metrics.total_overlap_extent
        ));
        output.push_str(&format!(
            "average overlap extent: {:.2}\n",
            metrics.average_overlap_extent
        ));
        output.push_str(&format!("has overlap: {}\n", metrics.has_overlap()));
        output.push('\n');
    }

    fn append_traversal_pressure_debug_output(
        &self,
        output: &mut String,
        result_bundle: &BenchmarkApplicationResultBundle,
    ) {
        let Some(first_baseline_report) = result_bundle.report.baseline_reports.first() else {
            return;
        };

        let aggregate = &first_baseline_report.report.aggregate;

        let visited_per_retained_leaf = ratio_or_zero(
            aggregate.total_fse_visited_nodes,
            aggregate.total_fse_retained_leaves,
        );
        let visited_per_reconstructed_record = ratio_or_zero(
            aggregate.total_fse_visited_nodes,
            aggregate.total_fse_reconstructed_records,
        );

        output.push_str("Traversal pressure summary\n");
        output.push_str("--------------------------\n");
        output.push_str(&format!(
            "total visited nodes: {}\n",
            aggregate.total_fse_visited_nodes
        ));
        output.push_str(&format!(
            "total retained leaves: {}\n",
            aggregate.total_fse_retained_leaves
        ));
        output.push_str(&format!(
            "total reconstructed records: {}\n",
            aggregate.total_fse_reconstructed_records
        ));
        output.push_str(&format!(
            "visited nodes per retained leaf: {:.2}\n",
            visited_per_retained_leaf
        ));
        output.push_str(&format!(
            "visited nodes per reconstructed record: {:.2}\n",
            visited_per_reconstructed_record
        ));
        output.push('\n');
    }

    fn append_low_selectivity_tree_gap_debug_output(
        &self,
        output: &mut String,
        result_bundle: &BenchmarkApplicationResultBundle,
    ) {
        let mut rendered_any_tree_baseline = false;

        output.push_str("Low-selectivity tree gap\n");
        output.push_str("------------------------\n");
        output.push_str("baseline | low bucket mean timing | low weighted candidate\n");

        for baseline_report in &result_bundle.report.baseline_reports {
            if !is_tree_baseline_name(&baseline_report.baseline_name) {
                continue;
            }

            let selectivity_summary =
                summarize_workloads_by_selectivity(&baseline_report.report.comparisons);

            if let Some(low_bucket) = selectivity_summary
                .bucket_summaries
                .iter()
                .find(|bucket| bucket.bucket == SelectivityBucket::Low)
            {
                output.push_str(&format!(
                    "{} | {:.2} | {:.2}\n",
                    baseline_report.baseline_name,
                    low_bucket.mean_timing_ratio,
                    low_bucket.weighted_candidate_ratio,
                ));
                rendered_any_tree_baseline = true;
            }
        }

        if !rendered_any_tree_baseline {
            output.push_str("none\n");
        }

        output.push('\n');
    }

    fn append_weakest_low_selectivity_workload_debug_output(
        &self,
        output: &mut String,
        result_bundle: &BenchmarkApplicationResultBundle,
    ) {
        let mut rendered_any_tree_baseline = false;

        output.push_str("Weakest low-selectivity workload\n");
        output.push_str("--------------------------------\n");
        output.push_str(
            "baseline | workload | mean timing | baseline avg | fse avg | visited nodes | fse records | baseline records\n",
        );

        for baseline_report in &result_bundle.report.baseline_reports {
            if !is_tree_baseline_name(&baseline_report.baseline_name) {
                continue;
            }

            let Some(weakest_low_workload) =
                weakest_low_selectivity_workload(&baseline_report.report.comparisons)
            else {
                continue;
            };

            let comparison = &weakest_low_workload.comparison;

            output.push_str(&format!(
                "{} | {} | {} | {} | {} | {} | {} | {}\n",
                baseline_report.baseline_name,
                weakest_low_workload.workload_name,
                format_f64_ratio(comparison.average_timing_ratio),
                format_duration_ascii(comparison.repeated_timing.baseline.average_elapsed),
                format_duration_ascii(comparison.repeated_timing.fse.average_elapsed),
                comparison.fse_stats.visited_nodes,
                comparison.fse_stats.reconstructed_records,
                comparison.baseline_stats.evaluated_records,
            ));
            rendered_any_tree_baseline = true;
        }

        if !rendered_any_tree_baseline {
            output.push_str("none\n");
        }

        output.push('\n');
    }

    fn append_boundary_workload_pressure_debug_output(
        &self,
        output: &mut String,
        result_bundle: &BenchmarkApplicationResultBundle,
    ) {
        let mut rendered_any_tree_baseline = false;

        output.push_str("Boundary workload pressure notes\n");
        output.push_str("--------------------------------\n");
        output.push_str(
            "baseline | workload | timing | baseline records | fse visited | fse retained | fse records | matched | candidate | nodes/record\n",
        );

        for baseline_report in &result_bundle.report.baseline_reports {
            if !is_tree_baseline_name(&baseline_report.baseline_name) {
                continue;
            }

            let Some(weakest_low_workload) =
                weakest_low_selectivity_workload(&baseline_report.report.comparisons)
            else {
                continue;
            };

            let comparison = &weakest_low_workload.comparison;
            let visited_per_reconstructed_record = ratio_or_zero(
                comparison.fse_stats.visited_nodes,
                comparison.fse_stats.reconstructed_records,
            );

            output.push_str(&format!(
                "{} | {} | {} | {} | {} | {} | {} | {} | {} | {:.2}\n",
                baseline_report.baseline_name,
                weakest_low_workload.workload_name,
                format_f64_ratio(comparison.average_timing_ratio),
                comparison.baseline_stats.evaluated_records,
                comparison.fse_stats.visited_nodes,
                comparison.fse_stats.retained_leaves,
                comparison.fse_stats.reconstructed_records,
                comparison.fse_stats.matched_records,
                format_scalar_ratio(comparison.candidate_ratio),
                visited_per_reconstructed_record,
            ));
            rendered_any_tree_baseline = true;
        }

        if !rendered_any_tree_baseline {
            output.push_str("none\n");
        }

        output.push('\n');
    }

    fn append_target_workload_retained_leaf_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        output.push_str("Target workload retained leaf details\n");
        output.push_str("-------------------------------------\n");

        let Some(workload) = context
            .workloads
            .iter()
            .find(|workload| workload.name == TARGET_BOUNDARY_WORKLOAD_NAME)
        else {
            output.push_str(&format!("workload: {}\n", TARGET_BOUNDARY_WORKLOAD_NAME));
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
        output.push_str(
            "leaf | coverage | records | matched | rejected | overlap volume | leaf volume | overlap ratio | bounds min | bounds max\n",
        );

        if traversal.retained_leaves.is_empty() {
            output.push_str("none\n");
            output.push('\n');
            return;
        }

        for retained_leaf in &traversal.retained_leaves {
            let node = &context.index.nodes[retained_leaf.node_id];
            let match_counts = retained_leaf_match_counts(node, &workload.query);
            let leaf_volume = node.bounds.volume();
            let overlap_volume = bounds_query_overlap_volume(&node.bounds, &workload.query);
            let overlap_ratio = scalar_ratio_or_zero(overlap_volume, leaf_volume);

            output.push_str(&format!(
                "{} | {} | {} | {} | {} | {:.2} | {:.2} | {:.4} | {} | {}\n",
                retained_leaf.node_id,
                retained_leaf_coverage_label(retained_leaf.coverage),
                node.stored_cardinality(),
                match_counts.matched_records,
                match_counts.rejected_records,
                overlap_volume,
                leaf_volume,
                overlap_ratio,
                format_bounds_min(&node.bounds),
                format_bounds_max(&node.bounds),
            ));
        }

        output.push('\n');
    }

    fn append_target_workload_retained_record_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        output.push_str("Target workload retained record details\n");
        output.push_str("---------------------------------------\n");

        let Some(workload) = context
            .workloads
            .iter()
            .find(|workload| workload.name == TARGET_BOUNDARY_WORKLOAD_NAME)
        else {
            output.push_str(&format!("workload: {}\n", TARGET_BOUNDARY_WORKLOAD_NAME));
            output.push_str("status: workload not found\n\n");
            return;
        };

        let traversal = traverse_with_stats(&context.index, &workload.query);

        output.push_str(&format!("workload: {}\n", workload.name));
        output.push_str("leaf | row | result | values\n");

        if traversal.retained_leaves.is_empty() {
            output.push_str("none\n");
            output.push('\n');
            return;
        }

        let mut scratch = Vec::with_capacity(workload.query.dimensions());

        for retained_leaf in &traversal.retained_leaves {
            let node = &context.index.nodes[retained_leaf.node_id];

            for row in 0..node.stored_cardinality() {
                reconstruct_row_into(node, row, &mut scratch);

                output.push_str(&format!(
                    "{} | {} | {} | {}\n",
                    retained_leaf.node_id,
                    row,
                    retained_record_result_label(&scratch, &workload.query),
                    format_coordinate_values(&scratch),
                ));
            }
        }

        output.push('\n');
    }

    fn append_debug_suite_terminal_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
        result_bundle: &BenchmarkApplicationResultBundle,
    ) {
        if context.has_multiple_baselines() {
            self.append_multi_baseline_debug_output(output, result_bundle);
        } else {
            self.append_single_baseline_debug_output(output, result_bundle);
        }
    }

    fn append_single_baseline_debug_output(
        &self,
        output: &mut String,
        result_bundle: &BenchmarkApplicationResultBundle,
    ) {
        let baseline_report = &result_bundle.report.baseline_reports[0];

        // debug mode gets the old wall of detail
        output.push_str(&render_suite_report(&baseline_report.report));
        self.append_selectivity_summary(output, &baseline_report.report.comparisons);
    }

    fn append_multi_baseline_debug_output(
        &self,
        output: &mut String,
        result_bundle: &BenchmarkApplicationResultBundle,
    ) {
        for baseline_report in &result_bundle.report.baseline_reports {
            output.push_str(&render_named_baseline_suite_report(
                &baseline_report.baseline_name,
                &baseline_report.report,
            ));

            // each baseline gets its own bucket view so timing doesnt get mixed
            self.append_selectivity_summary(output, &baseline_report.report.comparisons);
        }

        output.push_str(&render_multi_baseline_summary(
            &result_bundle.aggregate_summary,
        ));
    }

    fn append_selectivity_summary(
        &self,
        output: &mut String,
        workload_summaries: &[crate::benchmark::WorkloadComparisonSummary],
    ) {
        let selectivity_summary = summarize_workloads_by_selectivity(workload_summaries);

        output.push_str(&render_selectivity_bucketed_workload_summary(
            &selectivity_summary,
        ));
    }
}

/// Renders terminal output for a completed benchmark application run.
///
/// # Runtime Role
///
/// This helper preserves the previous function-level API while delegating
/// terminal formatting to `BenchmarkApplicationRenderer`.
pub fn render_benchmark_application_terminal_output(
    context: &BenchmarkApplicationContext,
    result_bundle: &BenchmarkApplicationResultBundle,
) -> String {
    BenchmarkApplicationRenderer::new().render_terminal_output(context, result_bundle)
}

const TARGET_BOUNDARY_WORKLOAD_NAME: &str = "cluster_boundary_range";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RetainedLeafMatchCounts {
    matched_records: usize,
    rejected_records: usize,
}

fn retained_leaf_match_counts(
    node: &PartitionNode,
    query: &QueryRegion,
) -> RetainedLeafMatchCounts {
    let mut scratch = Vec::with_capacity(query.dimensions());
    let mut matched_records = 0;

    for row in 0..node.stored_cardinality() {
        reconstruct_row_into(node, row, &mut scratch);

        if query.contains_values(&scratch) {
            matched_records += 1;
        }
    }

    RetainedLeafMatchCounts {
        matched_records,
        rejected_records: node.stored_cardinality() - matched_records,
    }
}

fn retained_record_result_label(values: &[Scalar], query: &QueryRegion) -> &'static str {
    if query.contains_values(values) {
        "match"
    } else {
        "reject"
    }
}

fn bounds_query_overlap_volume(bounds: &BoundingBox, query: &QueryRegion) -> Scalar {
    let dimensions = bounds
        .min
        .len()
        .min(bounds.max.len())
        .min(query.min.len())
        .min(query.max.len());

    if dimensions == 0 {
        return 0.0;
    }

    let mut volume = 1.0;

    for dimension in 0..dimensions {
        let overlap_min = bounds.min[dimension].max(query.min[dimension]);
        let overlap_max = bounds.max[dimension].min(query.max[dimension]);

        if overlap_max <= overlap_min {
            return 0.0;
        }

        volume *= overlap_max - overlap_min;
    }

    volume
}

fn scalar_ratio_or_zero(numerator: Scalar, denominator: Scalar) -> Scalar {
    if denominator == 0.0 {
        return 0.0;
    }

    numerator / denominator
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

fn render_scoreboard_row(summary: &BaselineAggregateSummary) -> String {
    format!(
        "{} | {} | {} | {} records | {} records | {}\n",
        summary.baseline_name,
        format_f64_ratio(summary.weighted_timing_ratio),
        timing_result_label(summary.weighted_timing_ratio),
        summary.total_baseline_evaluated_records,
        summary.total_fse_reconstructed_records,
        format_scalar_ratio(summary.weighted_candidate_ratio),
    )
}

fn render_best_relative_result(summary: &MultiBaselineAggregateSummary) -> String {
    match summary.baseline_summaries.iter().max_by(|left, right| {
        left.weighted_timing_ratio
            .partial_cmp(&right.weighted_timing_ratio)
            .unwrap_or(std::cmp::Ordering::Equal)
    }) {
        Some(best) => format!(
            "Best relative result:\n  {} at {}x\n",
            best.baseline_name,
            format_f64_ratio(best.weighted_timing_ratio)
        ),
        None => "Best relative result:\n  none\n".to_string(),
    }
}

fn render_scoreboard_diagnosis(summary: &MultiBaselineAggregateSummary) -> String {
    let mut output = String::new();

    output.push_str("Diagnosis:\n");

    if summary.baseline_summaries.is_empty() {
        output.push_str("  no baselines were run\n");
        return output;
    }

    if summary
        .baseline_summaries
        .iter()
        .any(|baseline| baseline.weighted_timing_ratio > 1.0)
    {
        output.push_str("  FSE beat at least one selected baseline\n");
    } else {
        output.push_str("  FSE did not beat any selected baseline\n");
    }

    if let Some(flat_scan) = summary
        .baseline_summaries
        .iter()
        .find(|baseline| baseline.baseline_name == "flat_scan")
    {
        if flat_scan.weighted_candidate_ratio < 1.0 {
            output.push_str("  FSE reduces candidate work compared to flat scan\n");
        } else {
            output.push_str("  FSE does not reduce candidate work compared to flat scan\n");
        }
    }

    let tree_baselines: Vec<&BaselineAggregateSummary> = summary
        .baseline_summaries
        .iter()
        .filter(|baseline| is_tree_baseline_name(&baseline.baseline_name))
        .collect();

    if !tree_baselines.is_empty()
        && tree_baselines
            .iter()
            .all(|baseline| baseline.weighted_candidate_ratio >= 1.0)
    {
        output.push_str("  FSE candidate work is not below the tree baselines yet\n");
    }

    output
}

fn next_target_message(overview: &BenchmarkRunOverview) -> &'static str {
    if overview.target_leaf_size <= 4 {
        return "compare tighter 4/8 geometry against traversal overhead";
    }

    if overview.target_leaf_size == overview.max_leaf_size {
        return "compare 8/8 timing against 4/8 candidate reduction";
    }

    "compare leaf policy against candidate ratio"
}

fn weakest_low_selectivity_workload(
    workload_summaries: &[crate::benchmark::WorkloadComparisonSummary],
) -> Option<&crate::benchmark::WorkloadComparisonSummary> {
    workload_summaries
        .iter()
        .filter(|summary| {
            SelectivityBucket::from_candidate_ratio(summary.comparison.candidate_ratio)
                == SelectivityBucket::Low
        })
        .min_by(|left, right| {
            left.comparison
                .average_timing_ratio
                .partial_cmp(&right.comparison.average_timing_ratio)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn is_tree_baseline_name(baseline_name: &str) -> bool {
    baseline_name == "kd_tree" || baseline_name == "r_tree"
}

fn timing_result_label(weighted_timing_ratio: f64) -> &'static str {
    if weighted_timing_ratio > 1.0 {
        "win"
    } else if weighted_timing_ratio == 1.0 {
        "tie"
    } else {
        "loss"
    }
}

fn format_scalar_ratio(value: crate::math::Scalar) -> String {
    format!("{:.2}", value)
}

fn format_f64_ratio(value: f64) -> String {
    format!("{:.2}", value)
}

fn ratio_or_zero(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        return 0.0;
    }

    numerator as f64 / denominator as f64
}
