//! Per-workload CSV rows.

use crate::benchmark::runner::{BaselineBenchmarkSuiteReport, MultiBaselineBenchmarkSuiteReport};

use super::document::{
    csv_document, format_ratio, header_fields_with_metadata, value_fields_with_metadata,
};
use super::metadata::BenchmarkCsvMetadata;

/// Converts a multi-baseline benchmark report into per-workload CSV text.
pub fn multi_baseline_workload_report_to_csv(report: &MultiBaselineBenchmarkSuiteReport) -> String {
    csv_document(workload_header_fields(), workload_value_rows(report))
}

/// Converts a multi-baseline benchmark report into per-workload CSV text with run metadata.
pub fn multi_baseline_workload_report_to_csv_with_metadata(
    metadata: &BenchmarkCsvMetadata,
    report: &MultiBaselineBenchmarkSuiteReport,
) -> String {
    csv_document(
        header_fields_with_metadata(workload_header_fields()),
        workload_value_rows(report)
            .into_iter()
            .map(|fields| value_fields_with_metadata(metadata, fields)),
    )
}

fn workload_value_rows(report: &MultiBaselineBenchmarkSuiteReport) -> Vec<Vec<String>> {
    let mut rows = Vec::new();

    for baseline_report in &report.baseline_reports {
        append_workload_value_rows(&mut rows, baseline_report);
    }

    rows
}

fn append_workload_value_rows(
    rows: &mut Vec<Vec<String>>,
    baseline_report: &BaselineBenchmarkSuiteReport,
) {
    for (summary, pruning_report) in baseline_report
        .report
        .comparisons
        .iter()
        .zip(&baseline_report.report.pruning_reports)
    {
        let comparison = &summary.comparison;
        let pruning = &pruning_report.pruning;

        rows.push(vec![
            baseline_report.baseline_name.clone(),
            comparison.labels.baseline_label.clone(),
            comparison.labels.comparison_label.clone(),
            summary.workload_name.clone(),
            comparison.baseline_stats.evaluated_records.to_string(),
            comparison.baseline_stats.matched_records.to_string(),
            comparison.fse_stats.visited_nodes.to_string(),
            comparison.fse_stats.retained_leaves.to_string(),
            comparison.fse_stats.reconstructed_records.to_string(),
            comparison.fse_stats.matched_records.to_string(),
            comparison.avoided_reconstructions.to_string(),
            format_ratio(comparison.reconstruction_avoidance_ratio as f64),
            format_ratio(comparison.candidate_ratio as f64),
            format_ratio(comparison.retained_leaf_ratio as f64),
            format_ratio(pruning.record_pruning_efficiency as f64),
            format_ratio(pruning.leaf_pruning_efficiency as f64),
            format_ratio(comparison.single_run_timing_ratio),
            format_ratio(comparison.average_timing_ratio),
            comparison.timing.baseline_elapsed.as_nanos().to_string(),
            comparison.timing.fse_elapsed.as_nanos().to_string(),
            comparison
                .repeated_timing
                .baseline
                .average_elapsed
                .as_nanos()
                .to_string(),
            comparison
                .repeated_timing
                .fse
                .average_elapsed
                .as_nanos()
                .to_string(),
            comparison.count_only_stats.visited_nodes.to_string(),
            comparison.count_only_stats.retained_leaves.to_string(),
            comparison
                .count_only_stats
                .reconstructed_records
                .to_string(),
            comparison.count_only_stats.matched_records.to_string(),
            comparison
                .count_only_repeated_timing
                .average_elapsed
                .as_nanos()
                .to_string(),
            comparison
                .estimated_owned_result_overhead
                .as_nanos()
                .to_string(),
            format_ratio(comparison.count_only_speedup_ratio),
            (comparison.count_only_stats == comparison.fse_stats).to_string(),
            comparison.reference_stats.visited_nodes.to_string(),
            comparison.reference_stats.retained_leaves.to_string(),
            comparison.reference_stats.reconstructed_records.to_string(),
            comparison.reference_stats.matched_records.to_string(),
            comparison
                .reference_repeated_timing
                .average_elapsed
                .as_nanos()
                .to_string(),
            comparison
                .estimated_owned_vs_reference_overhead
                .as_nanos()
                .to_string(),
            format_ratio(comparison.reference_result_speedup_ratio),
            (comparison.reference_stats == comparison.count_only_stats).to_string(),
            comparison.reusable_owned_stats.visited_nodes.to_string(),
            comparison.reusable_owned_stats.retained_leaves.to_string(),
            comparison
                .reusable_owned_stats
                .reconstructed_records
                .to_string(),
            comparison.reusable_owned_stats.matched_records.to_string(),
            comparison
                .reusable_owned_repeated_timing
                .average_elapsed
                .as_nanos()
                .to_string(),
            comparison
                .estimated_fresh_vs_reusable_owned_overhead
                .as_nanos()
                .to_string(),
            format_ratio(comparison.reusable_owned_result_speedup_ratio),
            (comparison.reusable_owned_stats == comparison.fse_stats).to_string(),
        ]);
    }
}

fn workload_header_fields() -> Vec<&'static str> {
    vec![
        "baseline_name",
        "baseline_label",
        "comparison_label",
        "workload_name",
        "baseline_evaluated_records",
        "baseline_matched_records",
        "fse_visited_nodes",
        "fse_retained_leaves",
        "fse_reconstructed_records",
        "fse_matched_records",
        "avoided_reconstructions",
        "reconstruction_avoidance_ratio",
        "candidate_ratio",
        "retained_leaf_ratio",
        "record_pruning_efficiency",
        "leaf_pruning_efficiency",
        "single_run_timing_ratio",
        "average_timing_ratio",
        "baseline_elapsed_ns",
        "fse_elapsed_ns",
        "baseline_average_elapsed_ns",
        "fse_average_elapsed_ns",
        "count_only_visited_nodes",
        "count_only_retained_leaves",
        "count_only_reconstructed_records",
        "count_only_matched_records",
        "count_only_average_elapsed_ns",
        "estimated_owned_result_overhead_ns",
        "count_only_speedup_ratio",
        "count_only_stats_match_owned",
        "reference_visited_nodes",
        "reference_retained_leaves",
        "reference_reconstructed_records",
        "reference_matched_records",
        "reference_average_elapsed_ns",
        "estimated_owned_vs_reference_overhead_ns",
        "reference_result_speedup_ratio",
        "reference_stats_match_count_only",
        "reusable_owned_visited_nodes",
        "reusable_owned_retained_leaves",
        "reusable_owned_reconstructed_records",
        "reusable_owned_matched_records",
        "reusable_owned_average_elapsed_ns",
        "estimated_fresh_vs_reusable_owned_overhead_ns",
        "reusable_owned_result_speedup_ratio",
        "reusable_owned_stats_match_owned",
    ]
}
