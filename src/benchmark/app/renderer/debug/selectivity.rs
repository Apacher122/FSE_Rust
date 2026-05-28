//! Selectivity-focused debug rendering.

use super::super::BenchmarkApplicationRenderer;
use super::super::helpers::{is_tree_baseline_name, weakest_low_selectivity_workload};
use crate::benchmark::formatting::{format_f64_fixed_2, format_scalar_fixed_2};
use crate::benchmark::math::f64_ratio_or_zero;
use crate::benchmark::reports::output::format_duration_ascii;
use crate::benchmark::reports::{SelectivityBucket, summarize_workloads_by_selectivity};

use super::super::super::result_bundle::BenchmarkApplicationResultBundle;

impl BenchmarkApplicationRenderer {
    pub(super) fn append_low_selectivity_tree_gap_debug_output(
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

    pub(super) fn append_weakest_low_selectivity_workload_debug_output(
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

    pub(super) fn append_boundary_workload_pressure_debug_output(
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
}
