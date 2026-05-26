//! Compact benchmark summary rendering.

use super::BenchmarkApplicationRenderer;
use super::helpers::is_tree_baseline_name;
use crate::benchmark::formatting::{format_f64_fixed_2, format_scalar_fixed_2};
use crate::benchmark::reports::{
    BaselineAggregateSummary, BenchmarkRunOverview, MultiBaselineAggregateSummary,
};

impl BenchmarkApplicationRenderer {
    pub(super) fn render_summary_terminal_output(
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
}

fn render_scoreboard_row(summary: &BaselineAggregateSummary) -> String {
    format!(
        "{} | {} | {} | {} records | {} records | {}\n",
        summary.baseline_name,
        format_f64_fixed_2(summary.weighted_timing_ratio),
        timing_result_label(summary.weighted_timing_ratio),
        summary.total_baseline_evaluated_records,
        summary.total_fse_reconstructed_records,
        format_scalar_fixed_2(summary.weighted_candidate_ratio),
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
            format_f64_fixed_2(best.weighted_timing_ratio)
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

fn timing_result_label(weighted_timing_ratio: f64) -> &'static str {
    if weighted_timing_ratio > 1.0 {
        "win"
    } else if weighted_timing_ratio == 1.0 {
        "tie"
    } else {
        "loss"
    }
}
