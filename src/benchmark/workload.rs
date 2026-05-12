//! Reusable benchmark query workloads.

use crate::query::QueryRegion;

/// Named query case used for repeatable benchmark and demo execution.
///
/// # Runtime Role
///
/// `QueryWorkloadCase` gives examples and benchmark code a stable way to run
/// multiple query shapes against the same dataset.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryWorkloadCase {
    /// Human-readable workload name.
    pub name: String,

    /// Query region executed for this workload case.
    pub query: QueryRegion,
}

impl QueryWorkloadCase {
    /// Creates a named workload case.
    pub fn new(name: impl Into<String>, query: QueryRegion) -> Self {
        Self {
            name: name.into(),
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
/// - boundary-crossing query
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
        QueryWorkloadCase::new(
            "cluster_boundary_range",
            QueryRegion::new(vec![18.0, 18.0], vec![52.0, 52.0]),
        ),
    ]
}
