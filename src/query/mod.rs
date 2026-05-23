//! Query execution components.
//!
//! This module contains query region definitions and the staged execution logic
//! used by the FSE runtime.

pub mod evaluator;
pub mod execution;
pub mod reconstruction;
pub mod region;
pub mod traversal;

pub use evaluator::evaluate_query;
pub use execution::{
    QueryCountReport, QueryExecutionMode, QueryExecutionOptions, QueryExecutionReport,
    QueryExecutionStats, count_query_matches, count_query_matches_with_stats, execute_query,
    execute_query_with_options, execute_query_with_stats, execute_query_with_stats_and_options,
};

pub(crate) use execution::execute_retained_leaf_batch_for_diagnostics;
pub use reconstruction::{reconstruct_partition, reconstruct_point, reconstruct_row_into};
pub use region::QueryRegion;
pub use traversal::{
    QueryTraversalReport, QueryTraversalStats, RetainedLeaf, RetainedLeafCoverage, traverse,
    traverse_with_stats,
};
