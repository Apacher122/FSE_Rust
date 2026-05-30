//! Query execution report types.
//!
//! This module groups public query reports, internal retained-leaf reports, and
//! result capacity policy behind the same `query::execution::reports` boundary
//! that callers already use.

mod capacity;
mod public;
mod retained;

pub use public::{
    QueryCountReport, QueryExecutionReport, QueryExecutionStats, QueryExistenceReport,
    QueryReferenceReport, QueryResultReference,
};

pub(crate) use capacity::result_capacity_hint;

#[cfg(test)]
pub(crate) use capacity::MAX_RESULT_PREALLOCATION;

pub(crate) use retained::{RetainedLeafBatchExecutionReport, RetainedLeafExecutionReport};
