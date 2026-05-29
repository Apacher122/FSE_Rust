//! Retained-leaf reusable result-slot writers.

use crate::math::{Scalar, Vector};

use super::batch::RetainedLeafBatchExecutionReport;

impl RetainedLeafBatchExecutionReport {
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

        // this cursor is the real len for the active query
        self.result_len += 1;
    }
}
