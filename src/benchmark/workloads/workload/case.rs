//! Named benchmark query workload case.

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
