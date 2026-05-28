//! Workload comparison summaries.
//!
//! This module owns per-workload comparison summaries and aggregate benchmark
//! metrics. Type definitions, workload execution, and aggregate calculation are
//! split by responsibility to keep the reporting layer easier to scan.

mod aggregate;
mod collection;
mod types;

pub use aggregate::aggregate_workload_metrics;
pub use collection::summarize_workload_comparisons;
pub use types::{AggregateWorkloadMetrics, WorkloadComparisonSummary};
