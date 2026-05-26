//! Query result capacity policy.

/// Maximum number of result slots preallocated before exact evaluation.
///
/// # Runtime Role
///
/// Query execution can know the retained candidate count before exact filtering,
/// but that count is only an upper bound on final matches. This cap keeps
/// selective queries from allocating a large result buffer just because a
/// conservative bounding region retained many candidates.
pub(crate) const MAX_RESULT_PREALLOCATION: usize = 4096;

/// Returns a bounded capacity hint for final query results.
///
/// # Runtime Role
///
/// The retained candidate count is an upper bound, not a prediction of final
/// matches. This helper avoids repeated result-vector growth for common result
/// sets while avoiding huge allocations for conservative retained partitions.
pub(crate) fn result_capacity_hint(retained_candidate_count: usize) -> usize {
    retained_candidate_count.min(MAX_RESULT_PREALLOCATION)
}
