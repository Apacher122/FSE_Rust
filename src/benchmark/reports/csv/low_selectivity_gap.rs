//! Low-selectivity tree-gap CSV rows.

use crate::benchmark::baselines::BaselineKind;
use crate::benchmark::reports::selectivity::{
    SelectivityBucket, summarize_workloads_by_selectivity,
};
use crate::benchmark::runner::MultiBaselineBenchmarkSuiteReport;

use super::baseline_footprint::{
    baseline_footprint_header_fields, baseline_footprint_value_fields,
};
use super::document::{
    csv_document, format_ratio, header_fields_with_metadata, value_fields_with_metadata,
};
use super::metadata::BenchmarkCsvMetadata;

/// Converts low-selectivity tree-gap summaries into CSV text.
///
/// # Runtime Role
///
/// This export mirrors the debug report's low-selectivity tree-gap section so
/// KD-tree and R-tree low-bucket behavior can be tracked across commits without
/// parsing terminal text.
pub fn multi_baseline_low_selectivity_gap_to_csv(
    report: &MultiBaselineBenchmarkSuiteReport,
) -> String {
    csv_document(
        low_selectivity_gap_header_fields(),
        low_selectivity_gap_value_rows(report),
    )
}

/// Converts low-selectivity tree-gap summaries into CSV text with run metadata.
pub fn multi_baseline_low_selectivity_gap_to_csv_with_metadata(
    metadata: &BenchmarkCsvMetadata,
    report: &MultiBaselineBenchmarkSuiteReport,
) -> String {
    csv_document(
        header_fields_with_metadata(low_selectivity_gap_header_fields()),
        low_selectivity_gap_value_rows(report)
            .into_iter()
            .map(|fields| value_fields_with_metadata(metadata, fields)),
    )
}

fn low_selectivity_gap_value_rows(report: &MultiBaselineBenchmarkSuiteReport) -> Vec<Vec<String>> {
    let mut rows = Vec::new();

    for baseline_report in &report.baseline_reports {
        if !matches!(
            baseline_report.baseline_kind,
            BaselineKind::KdTree | BaselineKind::RTree
        ) {
            continue;
        }

        let selectivity_summary =
            summarize_workloads_by_selectivity(&baseline_report.report.comparisons);

        let Some(low_bucket) = selectivity_summary.bucket_summary(SelectivityBucket::Low) else {
            continue;
        };

        let labels = baseline_report
            .report
            .comparisons
            .first()
            .map(|summary| summary.comparison.labels.clone());

        let baseline_label = labels
            .as_ref()
            .map(|labels| labels.baseline_label.clone())
            .unwrap_or_else(|| baseline_report.baseline_name.clone());

        let comparison_label = labels
            .map(|labels| labels.comparison_label)
            .unwrap_or_else(|| format!("{} vs FSE", baseline_label));

        let baseline_footprint = baseline_report
            .report
            .comparisons
            .first()
            .map(|summary| summary.comparison.baseline_footprint);

        let mut fields = vec![
            baseline_report.baseline_name.clone(),
            baseline_label,
            comparison_label,
        ];

        if let Some(baseline_footprint) = baseline_footprint {
            fields.extend(baseline_footprint_value_fields(&baseline_footprint));
        }

        fields.extend([
            low_bucket.workload_count.to_string(),
            low_bucket.total_baseline_evaluated_records.to_string(),
            low_bucket.total_fse_reconstructed_records.to_string(),
            low_bucket.total_avoided_reconstructions.to_string(),
            format_ratio(low_bucket.average_candidate_ratio as f64),
            format_ratio(low_bucket.weighted_candidate_ratio as f64),
            format_ratio(low_bucket.average_reconstruction_avoidance_ratio as f64),
            format_ratio(low_bucket.weighted_reconstruction_avoidance_ratio as f64),
            format_ratio(low_bucket.mean_timing_ratio),
        ]);

        rows.push(fields);
    }

    rows
}

fn low_selectivity_gap_header_fields() -> Vec<&'static str> {
    let mut fields = vec!["baseline_name", "baseline_label", "comparison_label"];

    fields.extend(baseline_footprint_header_fields());

    fields.extend([
        "low_workload_count",
        "low_baseline_evaluated_records",
        "low_fse_reconstructed_records",
        "low_avoided_reconstructions",
        "low_average_candidate_ratio",
        "low_weighted_candidate_ratio",
        "low_average_reconstruction_avoidance_ratio",
        "low_weighted_reconstruction_avoidance_ratio",
        "low_mean_timing_ratio",
    ]);

    fields
}
