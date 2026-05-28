//! Detailed suite debug rendering.

use super::super::BenchmarkApplicationRenderer;
use crate::benchmark::WorkloadComparisonSummary;
use crate::benchmark::reports::{
    render_multi_baseline_summary, render_named_baseline_suite_report,
    render_selectivity_bucketed_workload_summary, render_suite_report,
    summarize_workloads_by_selectivity,
};

use super::super::super::context::BenchmarkApplicationContext;
use super::super::super::result_bundle::BenchmarkApplicationResultBundle;

impl BenchmarkApplicationRenderer {
    pub(super) fn append_debug_suite_terminal_output(
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
