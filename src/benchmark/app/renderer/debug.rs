//! Debug benchmark terminal rendering.

use super::BenchmarkApplicationRenderer;
use super::helpers::{is_tree_baseline_name, weakest_low_selectivity_workload};
use crate::benchmark::WorkloadComparisonSummary;
use crate::benchmark::formatting::{format_f64_fixed_2, format_scalar_fixed_2};
use crate::benchmark::math::f64_ratio_or_zero;
use crate::benchmark::reports::output::format_duration_ascii;
use crate::benchmark::reports::{
    SelectivityBucket, render_benchmark_overview, render_multi_baseline_summary,
    render_named_baseline_suite_report, render_selectivity_bucketed_workload_summary,
    render_suite_report, summarize_workloads_by_selectivity,
};
use crate::build::sibling_overlap_metrics;

use super::super::context::BenchmarkApplicationContext;
use super::super::result_bundle::BenchmarkApplicationResultBundle;

impl BenchmarkApplicationRenderer {
    pub(super) fn render_debug_terminal_output(
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
        self.append_target_workload_resultless_timing_debug_output(&mut output, context);
        self.append_target_workload_count_only_comparison_debug_output(&mut output, context);
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

        let visited_per_retained_leaf = f64_ratio_or_zero(
            aggregate.total_fse_visited_nodes,
            aggregate.total_fse_retained_leaves,
        );
        let visited_per_reconstructed_record = f64_ratio_or_zero(
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
                format_f64_fixed_2(comparison.average_timing_ratio),
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
            let visited_per_reconstructed_record = f64_ratio_or_zero(
                comparison.fse_stats.visited_nodes,
                comparison.fse_stats.reconstructed_records,
            );

            output.push_str(&format!(
                "{} | {} | {} | {} | {} | {} | {} | {} | {} | {:.2}\n",
                baseline_report.baseline_name,
                weakest_low_workload.workload_name,
                format_f64_fixed_2(comparison.average_timing_ratio),
                comparison.baseline_stats.evaluated_records,
                comparison.fse_stats.visited_nodes,
                comparison.fse_stats.retained_leaves,
                comparison.fse_stats.reconstructed_records,
                comparison.fse_stats.matched_records,
                format_scalar_fixed_2(comparison.candidate_ratio),
                visited_per_reconstructed_record,
            ));
            rendered_any_tree_baseline = true;
        }

        if !rendered_any_tree_baseline {
            output.push_str("none\n");
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
        workload_summaries: &[WorkloadComparisonSummary],
    ) {
        let selectivity_summary = summarize_workloads_by_selectivity(workload_summaries);

        output.push_str(&render_selectivity_bucketed_workload_summary(
            &selectivity_summary,
        ));
    }
}
