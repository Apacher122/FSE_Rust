//! Public reference-result reconstruction API.

use crate::math::{Scalar, Vector};
use crate::query::reconstruction::{reconstruct_point, reconstruct_row_into};
use crate::storage::FSEIndex;

use super::super::super::reports::QueryResultReference;
use super::validation::reference_leaf_node;

/// Reconstructs an owned row from an exact query result reference.
///
/// # Runtime Role
///
/// Reference-result queries return stable leaf/row identifiers instead of
/// materializing owned [`Vector`] values. This helper is the explicit
/// reconstruction seam for callers that later need one referenced row as owned
/// coordinates.
///
/// # Formal Reference
///
/// This applies the reconstruction operator
/// $\Phi_k(\Delta) = \mu_k + \Delta$ to the referenced residual row. It does
/// not rerun geometric pruning or exact predicate evaluation.
///
/// # Panics
///
/// Panics when the referenced node does not exist, does not identify a leaf
/// partition, or the referenced row is outside that leaf's residual storage.
pub fn reconstruct_query_result_reference(
    index: &FSEIndex,
    reference: QueryResultReference,
) -> Vector {
    let node = reference_leaf_node(index, reference);

    reconstruct_point(node, reference.row_index)
}

/// Reconstructs an exact query result reference into a caller-owned coordinate buffer.
///
/// # Runtime Role
///
/// This is the allocation-conscious reconstruction path for reference-result
/// rows. The caller owns `output` and may reuse it across many references.
///
/// # Formal Reference
///
/// This applies $\Phi_k(\Delta) = \mu_k + \Delta$ to the referenced residual
/// row while preserving the same row-level reconstruction semantics as
/// owned-result query execution.
///
/// # Panics
///
/// Panics under the same reference validation rules as
/// [`reconstruct_query_result_reference`].
pub fn reconstruct_query_result_reference_into(
    index: &FSEIndex,
    reference: QueryResultReference,
    output: &mut Vec<Scalar>,
) {
    let node = reference_leaf_node(index, reference);

    reconstruct_row_into(node, reference.row_index, output);
}

/// Reconstructs owned rows from exact query result references.
///
/// # Runtime Role
///
/// This helper reconstructs a reference-result batch only when a caller chooses
/// to materialize the referenced rows. Query execution can still return
/// references without paying owned [`Vector`] materialization cost up front.
///
/// # Formal Reference
///
/// Each reference is reconstructed by applying
/// $\Phi_k(\Delta) = \mu_k + \Delta$ to its referenced residual row.
///
/// # Panics
///
/// Panics when any reference is invalid under the same rules as
/// [`reconstruct_query_result_reference`].
pub fn reconstruct_query_result_references(
    index: &FSEIndex,
    references: &[QueryResultReference],
) -> Vec<Vector> {
    let mut results = Vec::with_capacity(references.len());

    reconstruct_query_result_references_into(index, references, &mut results);

    results
}

/// Reconstructs exact query result references into a caller-owned result buffer.
///
/// # Runtime Role
///
/// This is the batch reconstruction equivalent of `execute_query_into` for
/// reference results. The caller owns the output vector and may reuse it across
/// repeated reconstruction calls. Existing result slots are overwritten in
/// place where possible so their inner coordinate buffers can be reused.
///
/// # Formal Reference
///
/// This function performs only deferred reconstruction. It does not rerun
/// geometric pruning or exact predicate evaluation because the supplied
/// references already represent accepted rows.
///
/// # Panics
///
/// Panics when any supplied reference is invalid.
pub fn reconstruct_query_result_references_into(
    index: &FSEIndex,
    references: &[QueryResultReference],
    results: &mut Vec<Vector>,
) {
    let target_capacity = references.len();

    if results.capacity() < target_capacity {
        results.reserve_exact(target_capacity - results.capacity());
    }

    let mut result_len = 0;

    for reference in references {
        reconstruct_reference_into_result_slot(index, *reference, results, result_len);
        result_len += 1;
    }

    results.truncate(result_len);
}

fn reconstruct_reference_into_result_slot(
    index: &FSEIndex,
    reference: QueryResultReference,
    results: &mut Vec<Vector>,
    slot_index: usize,
) {
    let node = reference_leaf_node(index, reference);

    if slot_index < results.len() {
        reconstruct_row_into(node, reference.row_index, &mut results[slot_index].values);
    } else {
        results.push(reconstruct_point(node, reference.row_index));
    }
}
