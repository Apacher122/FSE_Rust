//! Terminal rendering for benchmark application output.

use super::context::BenchmarkApplicationContext;
use super::result_bundle::BenchmarkApplicationResultBundle;
use crate::benchmark::reports::output::format_duration_ascii;
use crate::benchmark::reports::{
    BaselineAggregateSummary, BenchmarkRunOverview, MultiBaselineAggregateSummary,
    SelectivityBucket, duration_ratio, measure_repeated, render_benchmark_overview,
    render_multi_baseline_summary, render_named_baseline_suite_report,
    render_selectivity_bucketed_workload_summary, render_suite_report,
    summarize_workloads_by_selectivity,
};
use crate::build::sibling_overlap_metrics;
use crate::math::{BoundingBox, Scalar, Vector};
use crate::query::{
    QueryRegion, RetainedLeaf, RetainedLeafCoverage, execute_query_with_stats_and_options,
    execute_retained_leaf_batch_for_diagnostics, traverse_with_stats,
};
use crate::storage::{LeafReconstructionShape, PartitionNode};

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
            &result_bundle.overview,
            &result_bundle.aggregate_summary,
        )
    }

    fn render_summary_terminal_output(
        &self,
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
        self.append_sibling_overlap_debug_output(&mut output, context);
        self.append_traversal_pressure_debug_output(&mut output, result_bundle);
        self.append_low_selectivity_tree_gap_debug_output(&mut output, result_bundle);
        self.append_weakest_low_selectivity_workload_debug_output(&mut output, result_bundle);
        self.append_boundary_workload_pressure_debug_output(&mut output, result_bundle);
        // keep this as debug-only until the boundary data says what to optimize
        self.append_target_workload_retained_leaf_debug_output(&mut output, context);
        self.append_target_workload_stage_timing_debug_output(&mut output, context);
        self.append_target_workload_reconstruction_timing_debug_output(&mut output, context);
        self.append_target_workload_retained_execution_phase_debug_output(&mut output, context);
        self.append_target_workload_retained_allocation_debug_output(&mut output, context);
        self.append_debug_suite_terminal_output(&mut output, context, result_bundle);

        output
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

    fn append_target_workload_stage_timing_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        output.push_str("Target workload stage timing estimate\n");
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

    fn append_target_workload_reconstruction_timing_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        output.push_str("Target workload reconstruction timing estimate\n");
        output.push_str("----------------------------------------------\n");

        let Some(workload) = context
            .workloads
            .iter()
            .find(|workload| workload.name == TARGET_BOUNDARY_WORKLOAD_NAME)
        else {
            output.push_str(&format!("workload: {}\n", TARGET_BOUNDARY_WORKLOAD_NAME));
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

    fn append_target_workload_retained_execution_phase_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        output.push_str("Target workload retained execution phase estimate\n");
        output.push_str("-------------------------------------------------\n");

        let Some(workload) = context
            .workloads
            .iter()
            .find(|workload| workload.name == TARGET_BOUNDARY_WORKLOAD_NAME)
        else {
            output.push_str(&format!("workload: {}\n", TARGET_BOUNDARY_WORKLOAD_NAME));
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

    fn append_target_workload_retained_allocation_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        output.push_str("Target workload retained allocation estimate\n");
        output.push_str("--------------------------------------------\n");

        let Some(workload) = context
            .workloads
            .iter()
            .find(|workload| workload.name == TARGET_BOUNDARY_WORKLOAD_NAME)
        else {
            output.push_str(&format!("workload: {}\n", TARGET_BOUNDARY_WORKLOAD_NAME));
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

fn format_percent_ratio(value: f64) -> String {
    if value.is_infinite() {
        return "inf".to_string();
    }

    format!("{:.2}%", value * 100.0)
}

fn ratio_or_zero(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        return 0.0;
    }

    numerator as f64 / denominator as f64
}
