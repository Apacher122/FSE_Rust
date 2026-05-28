//! Single-baseline suite terminal rendering.

use std::fmt::Write;

use crate::benchmark::BenchmarkSuiteReport;

use self::aggregate::render_aggregate_metrics;
use self::workload::render_workload_comparison;

mod aggregate;
mod workload;

/// Renders a named baseline suite section.
pub fn render_named_baseline_suite_report(
    baseline_name: &str,
    report: &BenchmarkSuiteReport,
) -> String {
    let mut output = String::new();

    writeln!(output, "Baseline suite: {}", baseline_name).unwrap();
    writeln!(output, "----------------").unwrap();
    output.push_str(&render_suite_report(report));
    writeln!(output).unwrap();

    output
}

/// Renders a benchmark suite report for one baseline.
pub fn render_suite_report(report: &BenchmarkSuiteReport) -> String {
    let mut output = String::new();

    for (summary, pruning_report) in report.comparisons.iter().zip(&report.pruning_reports) {
        render_workload_comparison(summary, pruning_report, &mut output);
    }

    render_aggregate_metrics(report, &mut output);

    output
}
