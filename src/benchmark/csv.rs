//! CSV export utilities for benchmark reports.

use crate::benchmark::MultiBaselineAggregateSummary;

/// Converts a multi-baseline aggregate summary into CSV text.
///
/// # Runtime Role
///
/// This function provides a simple export format for benchmark aggregate data so
/// results can be inspected in spreadsheets, copied into notes, or used by later
/// plotting scripts.
pub fn multi_baseline_aggregate_summary_to_csv(summary: &MultiBaselineAggregateSummary) -> String {
    let mut rows = Vec::new();

    rows.push(csv_row(&[
        "baseline_name",
        "baseline_label",
        "comparison_label",
        "workload_count",
        "total_baseline_evaluated_records",
        "total_fse_reconstructed_records",
        "weighted_reconstruction_avoidance_ratio",
        "weighted_candidate_ratio",
        "mean_timing_ratio",
        "weighted_timing_ratio",
        "total_baseline_average_elapsed_ns",
        "total_fse_average_elapsed_ns",
    ]));

    for baseline in &summary.baseline_summaries {
        rows.push(csv_row(&[
            baseline.baseline_name.clone(),
            baseline.baseline_label.clone(),
            baseline.comparison_label.clone(),
            baseline.workload_count.to_string(),
            baseline.total_baseline_evaluated_records.to_string(),
            baseline.total_fse_reconstructed_records.to_string(),
            format_ratio(baseline.weighted_reconstruction_avoidance_ratio as f64),
            format_ratio(baseline.weighted_candidate_ratio as f64),
            format_ratio(baseline.mean_timing_ratio),
            format_ratio(baseline.weighted_timing_ratio),
            baseline
                .total_baseline_average_elapsed
                .as_nanos()
                .to_string(),
            baseline.total_fse_average_elapsed.as_nanos().to_string(),
        ]));
    }

    rows.join("\n")
}

fn csv_row<I, S>(fields: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    fields
        .into_iter()
        .map(|field| escape_csv_field(field.as_ref()))
        .collect::<Vec<String>>()
        .join(",")
}

fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        let escaped = field.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        field.to_string()
    }
}

fn format_ratio(value: f64) -> String {
    format!("{:.6}", value)
}
