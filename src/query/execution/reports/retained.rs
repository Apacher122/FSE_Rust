//! Internal retained-leaf execution report types.

use crate::math::{Scalar, Vector};

use super::capacity::result_capacity_hint;

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

/// Result of executing all retained leaf partitions.
///
/// # Runtime Role
///
/// This report aggregates the Stage II and Stage III work performed across a
/// retained leaf batch. It is intentionally separate from traversal statistics
/// so the retained-partition execution strategy can evolve independently.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RetainedLeafBatchExecutionReport {
    /// Exact matches produced by all retained leaves in the batch.
    pub(crate) results: Vec<Vector>,

    /// Number of accepted rows currently written into `results`.
    ///
    /// # Runtime Role
    ///
    /// Reusable owned-result execution keeps old `Vector` slots alive while a
    /// new query is being written. This cursor separates logical result length
    /// from the temporary backing buffer length so inner coordinate buffers can
    /// be reused before stale slots are truncated.
    pub(crate) result_len: usize,

    /// Number of records reconstructed across retained leaves.
    pub(crate) reconstructed_records: usize,

    /// Number of records that required exact predicate checks.
    ///
    /// # Runtime Role
    ///
    /// This field is test-only until predicate-check counts become part of
    /// benchmark-facing execution stats.
    #[cfg(test)]
    pub(crate) predicate_evaluated_records: usize,

    /// Number of records that matched the exact query predicate.
    pub(crate) matched_records: usize,
}

impl RetainedLeafBatchExecutionReport {
    /// Creates an empty batch report with bounded result capacity.
    ///
    /// # Runtime Role
    ///
    /// Serial retained-leaf execution streams directly into this report instead
    /// of allocating one local result vector per leaf.
    pub(crate) fn with_candidate_capacity(candidate_count: usize) -> Self {
        Self {
            results: Vec::with_capacity(result_capacity_hint(candidate_count)),
            result_len: 0,
            reconstructed_records: 0,
            #[cfg(test)]
            predicate_evaluated_records: 0,
            matched_records: 0,
        }
    }

    /// Creates an empty batch report from a caller-owned result buffer.
    ///
    /// # Runtime Role
    ///
    /// This constructor lets owned-result query execution reuse the outer
    /// `Vec<Vector>` allocation across repeated exact queries. Existing result
    /// slots are kept alive during execution so their inner coordinate buffers
    /// can be reused before stale rows are truncated.
    pub(crate) fn with_result_buffer(candidate_count: usize, mut results: Vec<Vector>) -> Self {
        let target_capacity = result_capacity_hint(candidate_count);

        if results.capacity() < target_capacity {
            results.reserve_exact(target_capacity - results.capacity());
        }

        Self {
            results,
            result_len: 0,
            reconstructed_records: 0,
            #[cfg(test)]
            predicate_evaluated_records: 0,
            matched_records: 0,
        }
    }

    /// Returns the number of accepted result rows written for the active query.
    pub(crate) fn accepted_result_count(&self) -> usize {
        self.result_len
    }

    /// Reserves outer result capacity without using stale reusable slots as live rows.
    ///
    /// # Runtime Role
    ///
    /// During reusable execution, `results.len()` may temporarily describe the
    /// previous query output. Capacity decisions for the active query must use
    /// `result_len` instead so stale rows do not distort the reserve calculation.
    pub(crate) fn reserve_additional_results(&mut self, incoming_len: usize) {
        let available_capacity = self.results.capacity().saturating_sub(self.result_len);

        if incoming_len > available_capacity {
            self.results
                .reserve_exact(incoming_len - available_capacity);
        }
    }

    /// Appends a one-dimensional owned result row.
    pub(crate) fn push_result_1d(&mut self, value_0: Scalar) {
        if self.result_len < self.results.len() {
            let values = &mut self.results[self.result_len].values;
            values.clear();

            if values.capacity() < 1 {
                values.reserve_exact(1 - values.capacity());
            }

            values.push(value_0);
        } else {
            self.results.push(Vector::new(vec![value_0]));
        }

        self.result_len += 1;
    }

    /// Appends a two-dimensional owned result row.
    pub(crate) fn push_result_2d(&mut self, value_0: Scalar, value_1: Scalar) {
        if self.result_len < self.results.len() {
            let values = &mut self.results[self.result_len].values;
            values.clear();

            if values.capacity() < 2 {
                values.reserve_exact(2 - values.capacity());
            }

            values.push(value_0);
            values.push(value_1);
        } else {
            self.results.push(Vector::new(vec![value_0, value_1]));
        }

        self.result_len += 1;
    }

    /// Appends an owned result row from an existing coordinate buffer.
    ///
    /// # Runtime Role
    ///
    /// When a reusable result slot exists, this copies the accepted coordinates
    /// into the old row allocation. When no reusable slot exists, it moves the
    /// scratch row into the result set and gives the caller a new scratch buffer.
    pub(crate) fn push_result_from_buffer(&mut self, values: &mut Vec<Scalar>, dimensions: usize) {
        debug_assert_eq!(
            values.len(),
            dimensions,
            "accepted row dimensionality should match the partition"
        );

        if self.result_len < self.results.len() {
            let reusable_values = &mut self.results[self.result_len].values;
            reusable_values.clear();

            if reusable_values.capacity() < dimensions {
                reusable_values.reserve_exact(dimensions - reusable_values.capacity());
            }

            reusable_values.extend_from_slice(&values[..dimensions]);
        } else {
            let accepted_values = std::mem::replace(values, Vec::with_capacity(dimensions));
            self.results.push(Vector::new(accepted_values));
        }

        self.result_len += 1;
    }

    /// Appends an owned result row from materialized coordinate values.
    pub(crate) fn push_result_values(&mut self, values: Vec<Scalar>) {
        if self.result_len < self.results.len() {
            let reusable_values = &mut self.results[self.result_len].values;
            reusable_values.clear();

            if reusable_values.capacity() < values.len() {
                reusable_values.reserve_exact(values.len() - reusable_values.capacity());
            }

            reusable_values.extend_from_slice(&values);
        } else {
            self.results.push(Vector::new(values));
        }

        self.result_len += 1;
    }

    /// Drops stale rows left over from a previous reusable query.
    pub(crate) fn truncate_to_accepted_results(&mut self) {
        self.results.truncate(self.result_len);
    }
}
