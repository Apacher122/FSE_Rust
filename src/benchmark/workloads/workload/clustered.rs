//! Clustered benchmark workload presets.

use crate::query::QueryRegion;

use super::case::QueryWorkloadCase;
use super::range::{RangeWorkloadConfig, generate_range_workload_cases};

/// Returns reusable query cases for the small deterministic clustered 2D dataset.
///
/// # Runtime Role
///
/// These cases cover different selectivity profiles for the 60-record demo
/// dataset.
pub fn clustered_workload_cases() -> Vec<QueryWorkloadCase> {
    let mut workloads = generate_range_workload_cases(&RangeWorkloadConfig::new(
        "cluster_range",
        3,
        vec![0.0, 0.0],
        vec![50.0, 50.0],
        vec![5.0, 5.0],
    ));

    // Keep a few named cases because they are easier to read in demo output.
    workloads.push(QueryWorkloadCase::new(
        "empty_far_range",
        QueryRegion::new(vec![200.0, 200.0], vec![220.0, 220.0]),
    ));

    workloads.push(QueryWorkloadCase::new(
        "full_dataset_range",
        QueryRegion::new(vec![-10.0, -10.0], vec![130.0, 130.0]),
    ));

    workloads.push(QueryWorkloadCase::new(
        "cluster_boundary_range",
        QueryRegion::new(vec![18.0, 18.0], vec![52.0, 52.0]),
    ));

    workloads
}

/// Returns reusable query cases for the large deterministic clustered 2D dataset.
///
/// # Runtime Role
///
/// These cases target the 10,000-record deterministic dataset whose cluster
/// origins are spaced by 1,000 units.
pub fn large_clustered_workload_cases() -> Vec<QueryWorkloadCase> {
    let mut workloads = generate_range_workload_cases(&RangeWorkloadConfig::new(
        "large_cluster_range",
        10,
        vec![0.0, 0.0],
        vec![1000.0, 1000.0],
        vec![25.0, 25.0],
    ));

    workloads.push(QueryWorkloadCase::new(
        "large_empty_far_range",
        QueryRegion::new(vec![20_000.0, 20_000.0], vec![21_000.0, 21_000.0]),
    ));

    workloads.push(QueryWorkloadCase::new(
        "large_full_dataset_range",
        QueryRegion::new(vec![-100.0, -100.0], vec![10_000.0, 10_000.0]),
    ));

    workloads.push(QueryWorkloadCase::new(
        "large_cross_cluster_boundary",
        QueryRegion::new(vec![490.0, 490.0], vec![1_025.0, 1_025.0]),
    ));

    workloads
}
