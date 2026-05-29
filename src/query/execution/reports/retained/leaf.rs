//! Single retained-leaf execution report.

use crate::math::Vector;

/// Result of executing one retained leaf partition.
///
/// # Runtime Role
///
/// This report isolates Stage II and Stage III work for a single retained leaf.
/// The structure is intentionally local to query execution so retained leaves
/// can be evaluated independently without changing query semantics.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RetainedLeafExecutionReport {
    /// Exact matches produced by this retained leaf.
    pub(crate) results: Vec<Vector>,

    /// Number of records reconstructed from this retained leaf.
    pub(crate) reconstructed_records: usize,

    /// Number of records that required exact predicate checks.
    ///
    /// # Runtime Role
    ///
    /// This field is test-only because current benchmark accounting does not
    /// expose predicate-check counts yet.
    #[cfg(test)]
    pub(crate) predicate_evaluated_records: usize,

    /// Number of records that matched the exact query predicate.
    pub(crate) matched_records: usize,
}
