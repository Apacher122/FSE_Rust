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
    QueryExecutionMode, QueryExecutionOptions, QueryExecutionReport, QueryExecutionStats,
    execute_query, execute_query_with_options, execute_query_with_stats,
    execute_query_with_stats_and_options,
};
pub use reconstruction::{reconstruct_partition, reconstruct_point, reconstruct_row_into};
pub use region::QueryRegion;
pub use traversal::{
    QueryTraversalReport, QueryTraversalStats, RetainedLeaf, RetainedLeafCoverage, traverse,
    traverse_with_stats,
};
