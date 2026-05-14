use std::time::Duration;

use crate::benchmark::{
    BaselineAggregateSummary, BaselineKind, MultiBaselineAggregateSummary,
    multi_baseline_aggregate_summary_to_csv,
};

#[test]
fn csv_export_includes_header() {
    let summary = MultiBaselineAggregateSummary::default();

    let csv = multi_baseline_aggregate_summary_to_csv(&summary);

    assert!(csv.starts_with("baseline_name,baseline_label,comparison_label,workload_count"));
}

#[test]
fn csv_export_includes_one_row_per_baseline_summary() {
    let summary = MultiBaselineAggregateSummary {
        baseline_summaries: vec![
            BaselineAggregateSummary {
                baseline_kind: BaselineKind::FlatScan,
                baseline_name: "flat_scan".to_string(),
                baseline_label: "Flat Scan".to_string(),
                comparison_label: "Flat Scan vs FSE".to_string(),
                workload_count: 3,
                total_baseline_evaluated_records: 300,
                total_fse_reconstructed_records: 75,
                weighted_reconstruction_avoidance_ratio: 0.75,
                weighted_candidate_ratio: 0.25,
                mean_timing_ratio: 1.5,
                weighted_timing_ratio: 1.25,
                total_baseline_average_elapsed: Duration::from_nanos(100),
                total_fse_average_elapsed: Duration::from_nanos(80),
            },
            BaselineAggregateSummary {
                baseline_kind: BaselineKind::KdTree,
                baseline_name: "kd_tree".to_string(),
                baseline_label: "KD-Tree".to_string(),
                comparison_label: "KD-Tree vs FSE".to_string(),
                workload_count: 3,
                total_baseline_evaluated_records: 120,
                total_fse_reconstructed_records: 75,
                weighted_reconstruction_avoidance_ratio: 0.375,
                weighted_candidate_ratio: 0.625,
                mean_timing_ratio: 0.8,
                weighted_timing_ratio: 0.9,
                total_baseline_average_elapsed: Duration::from_nanos(90),
                total_fse_average_elapsed: Duration::from_nanos(100),
            },
        ],
    };

    let csv = multi_baseline_aggregate_summary_to_csv(&summary);
    let rows: Vec<&str> = csv.lines().collect();

    assert_eq!(rows.len(), 3);
    assert!(rows[1].starts_with("flat_scan,Flat Scan,Flat Scan vs FSE,3"));
    assert!(rows[2].starts_with("kd_tree,KD-Tree,KD-Tree vs FSE,3"));
}

#[test]
fn csv_export_formats_ratios_with_fixed_precision() {
    let summary = MultiBaselineAggregateSummary {
        baseline_summaries: vec![BaselineAggregateSummary {
            baseline_kind: BaselineKind::FlatScan,
            baseline_name: "flat_scan".to_string(),
            baseline_label: "Flat Scan".to_string(),
            comparison_label: "Flat Scan vs FSE".to_string(),
            workload_count: 1,
            total_baseline_evaluated_records: 100,
            total_fse_reconstructed_records: 25,
            weighted_reconstruction_avoidance_ratio: 0.75,
            weighted_candidate_ratio: 0.25,
            mean_timing_ratio: 1.5,
            weighted_timing_ratio: 1.25,
            total_baseline_average_elapsed: Duration::from_nanos(100),
            total_fse_average_elapsed: Duration::from_nanos(80),
        }],
    };

    let csv = multi_baseline_aggregate_summary_to_csv(&summary);

    assert!(csv.contains("0.750000"));
    assert!(csv.contains("0.250000"));
    assert!(csv.contains("1.500000"));
    assert!(csv.contains("1.250000"));
}

#[test]
fn csv_export_escapes_fields_that_need_quotes() {
    let summary = MultiBaselineAggregateSummary {
        baseline_summaries: vec![BaselineAggregateSummary {
            baseline_kind: BaselineKind::FlatScan,
            baseline_name: "custom_baseline".to_string(),
            baseline_label: "Custom, Baseline".to_string(),
            comparison_label: "Custom \"Baseline\" vs FSE".to_string(),
            workload_count: 1,
            total_baseline_evaluated_records: 10,
            total_fse_reconstructed_records: 5,
            weighted_reconstruction_avoidance_ratio: 0.5,
            weighted_candidate_ratio: 0.5,
            mean_timing_ratio: 1.0,
            weighted_timing_ratio: 1.0,
            total_baseline_average_elapsed: Duration::from_nanos(10),
            total_fse_average_elapsed: Duration::from_nanos(10),
        }],
    };

    let csv = multi_baseline_aggregate_summary_to_csv(&summary);

    assert!(csv.contains("\"Custom, Baseline\""));
    assert!(csv.contains("\"Custom \"\"Baseline\"\" vs FSE\""));
}
