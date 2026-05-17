//! Terminal rendering for benchmark application output.

use super::context::BenchmarkApplicationContext;
use super::result_bundle::BenchmarkApplicationResultBundle;
use crate::benchmark::reports::{
    BaselineAggregateSummary, BenchmarkRunOverview, MultiBaselineAggregateSummary,
    render_benchmark_overview, render_multi_baseline_summary, render_named_baseline_suite_report,
    render_selectivity_bucketed_workload_summary, render_suite_report,
    summarize_workloads_by_selectivity,
};

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
        output.push_str("  tune retained-leaf execution threshold\n");

        output
    }

    fn render_debug_terminal_output(
        &self,
        context: &BenchmarkApplicationContext,
        result_bundle: &BenchmarkApplicationResultBundle,
    ) -> String {
        let mut output = String::new();

        output.push_str(&render_benchmark_overview(&result_bundle.overview));
        self.append_debug_suite_terminal_output(&mut output, context, result_bundle);

        output
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
        .filter(|baseline| {
            baseline.baseline_name == "kd_tree" || baseline.baseline_name == "r_tree"
        })
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
