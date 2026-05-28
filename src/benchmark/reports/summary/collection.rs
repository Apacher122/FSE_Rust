//! Workload comparison collection.

use crate::benchmark::{QueryWorkloadCase, compare_query_execution};
use crate::math::Vector;
use crate::storage::FSEIndex;

use super::types::WorkloadComparisonSummary;

/// Runs all workload cases and returns comparison summaries.
///
/// # Runtime Role
///
/// This function is used by demos and future benchmark harnesses to evaluate a
/// stable set of query workloads against a fixed dataset and FSE index.
///
/// # Panics
///
/// Panics if any FSE query result differs from the baseline result.
pub fn summarize_workload_comparisons(
    index: &FSEIndex,
    points: &[Vector],
    workloads: &[QueryWorkloadCase],
) -> Vec<WorkloadComparisonSummary> {
    workloads
        .iter()
        .map(|workload| WorkloadComparisonSummary {
            workload_name: workload.name.clone(),
            comparison: compare_query_execution(index, points, &workload.query),
        })
        .collect()
}
