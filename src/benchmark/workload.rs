//! Reusable benchmark query workloads.

use crate::query::QueryRegion;

pub struct QueryWorkloadCase {
    pub name: String,
    pub query: QueryRegion,
}

/// Named query case used for repeatable benchmark and demo execution.
///
/// # Runtime Role
///
/// `QueryWorkloadCase` gives examples and benchmark code a stable way to run
/// multiple query shapes against the same dataset.
impl QueryWorkloadCase {
    pub fn new(name: impl Into<String>, query: QueryRegion) -> Self {
        Self {
            // Human-readable workload name
            name: name.into(),
            // Query region executed for this workload case
            query,
        }
    }
}

/// Returns reusable query cases for the deterministic clustered 2D dataset.
///
/// # Runtime Role
///
/// These cases cover different selectivity profiles:
///
/// - selective cluster query
/// - empty query
/// - full-range query
pub fn clustered_workload_cases() -> Vec<QueryWorkloadCase> {
    vec![
        QueryWorkloadCase::new(
            "middle_cluster_selective",
            QueryRegion::new(vec![50.0, 50.0], vec![55.0, 55.0]),
        ),
        QueryWorkloadCase::new(
            "empty_far_range",
            QueryRegion::new(vec![200.0, 200.0], vec![220.0, 220.0]),
        ),
        QueryWorkloadCase::new(
            "full_dataset_range",
            QueryRegion::new(vec![-10.0, -10.0], vec![130.0, 130.0]),
        ),
    ]
}
