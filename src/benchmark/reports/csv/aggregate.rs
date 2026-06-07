//! Aggregate summary CSV rows.

use crate::benchmark::reports::multi_summary::{
    BaselineAggregateSummary, MultiBaselineAggregateSummary,
};

use super::baseline_footprint::{
    baseline_footprint_header_fields, baseline_footprint_value_fields,
};
use super::document::{
    csv_document, format_ratio, header_fields_with_metadata, value_fields_with_metadata,
};
use super::metadata::BenchmarkCsvMetadata;

/// Converts a multi-baseline aggregate summary into CSV text.
///
/// # Runtime Role
///
/// This function provides a simple export format for benchmark aggregate data so
/// results can be inspected in spreadsheets, copied into notes, or used by later
/// plotting scripts.
pub fn multi_baseline_aggregate_summary_to_csv(summary: &MultiBaselineAggregateSummary) -> String {
    csv_document(
        aggregate_header_fields(),
        summary
            .baseline_summaries
            .iter()
            .map(aggregate_value_fields),
    )
}

/// Converts a multi-baseline aggregate summary into CSV text with run metadata.
pub fn multi_baseline_aggregate_summary_to_csv_with_metadata(
    metadata: &BenchmarkCsvMetadata,
    summary: &MultiBaselineAggregateSummary,
) -> String {
    csv_document(
        header_fields_with_metadata(aggregate_header_fields()),
        summary
            .baseline_summaries
            .iter()
            .map(|baseline| value_fields_with_metadata(metadata, aggregate_value_fields(baseline))),
    )
}

fn aggregate_header_fields() -> Vec<&'static str> {
    let mut fields = vec!["baseline_name", "baseline_label", "comparison_label"];

    fields.extend(baseline_footprint_header_fields());

    fields.extend([
        "workload_count",
        "total_baseline_evaluated_records",
        "total_fse_reconstructed_records",
        "weighted_reconstruction_avoidance_ratio",
        "weighted_candidate_ratio",
        "mean_timing_ratio",
        "weighted_timing_ratio",
        "total_baseline_average_elapsed_ns",
        "total_fse_average_elapsed_ns",
    ]);

    fields
}

fn aggregate_value_fields(baseline: &BaselineAggregateSummary) -> Vec<String> {
    let mut fields = vec![
        baseline.baseline_name.clone(),
        baseline.baseline_label.clone(),
        baseline.comparison_label.clone(),
    ];

    fields.extend(baseline_footprint_value_fields(
        &baseline.baseline_footprint,
    ));

    fields.extend([
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
    ]);

    fields
}
