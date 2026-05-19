//! Retained-leaf execution helpers.

#[cfg(test)]
use crate::math::BoundingBox;
use crate::math::{Scalar, Vector};
#[cfg(test)]
use crate::query::reconstruction::validate_partition_reconstruction_shape;
use crate::query::reconstruction::{
    reconstruct_point_prevalidated, reconstruct_row_into_prevalidated,
};
use crate::query::{QueryRegion, RetainedLeaf, RetainedLeafCoverage};
use crate::storage::{FSEIndex, LeafReconstructionShape, PartitionNode};

use super::options::{QueryExecutionMode, QueryExecutionOptions};
use super::parallel::{
    execute_classified_retained_leaves_parallel_with_candidate_count,
    should_execute_retained_leaves_in_parallel,
};
use super::reports::{
    QueryExecutionStats, RetainedLeafBatchExecutionReport, RetainedLeafExecutionReport,
    result_capacity_hint,
};
use super::serial::execute_classified_retained_leaves_serial_with_candidate_count;

/// Executes Stage II and Stage III for all retained leaves using default options.
///
/// # Runtime Role
///
/// This helper preserves the default retained-leaf batch API while classifying
/// retained leaf identifiers before execution.
///
/// # Notes
///
/// This exists for internal tests that still exercise the id-based retained-leaf
/// API. Normal query execution consumes traversal-classified retained leaves.
#[cfg(test)]
pub(crate) fn execute_retained_leaves(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaf_ids: &[usize],
) -> RetainedLeafBatchExecutionReport {
    execute_retained_leaves_with_options(
        index,
        query,
        retained_leaf_ids,
        QueryExecutionOptions::default(),
    )
}

/// Executes Stage II and Stage III for retained leaf identifiers using explicit options.
///
/// # Runtime Role
///
/// This compatibility helper accepts retained leaf identifiers and performs the
/// coverage classification needed by the newer retained-leaf execution path.
#[cfg(test)]
pub(crate) fn execute_retained_leaves_with_options(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaf_ids: &[usize],
    options: QueryExecutionOptions,
) -> RetainedLeafBatchExecutionReport {
    let retained_leaves = classify_retained_leaf_ids(index, query, retained_leaf_ids);

    execute_classified_retained_leaves_with_options(index, query, &retained_leaves, options)
}

/// Executes retained leaf identifiers using deterministic serial iteration.
///
/// # Runtime Role
///
/// This preserves the older test/helper API by classifying leaf ids before
/// dispatching to the classified retained-leaf execution path.
#[cfg(test)]
pub(crate) fn execute_retained_leaves_serial(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaf_ids: &[usize],
) -> RetainedLeafBatchExecutionReport {
    let retained_leaves = classify_retained_leaf_ids(index, query, retained_leaf_ids);

    super::serial::execute_classified_retained_leaves_serial(index, query, &retained_leaves)
}

/// Executes retained leaf identifiers using Rayon-backed parallel iteration.
///
/// # Runtime Role
///
/// This preserves the older test/helper API by classifying leaf ids before
/// dispatching to the classified retained-leaf execution path.
#[cfg(test)]
pub(crate) fn execute_retained_leaves_parallel(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaf_ids: &[usize],
) -> RetainedLeafBatchExecutionReport {
    let retained_leaves = classify_retained_leaf_ids(index, query, retained_leaf_ids);

    super::parallel::execute_classified_retained_leaves_parallel(index, query, &retained_leaves)
}

/// Executes already classified retained leaves using explicit options.
///
/// # Runtime Role
///
/// This compatibility helper is test-only. The release query path supplies the
/// retained candidate count from traversal and uses
/// `execute_classified_retained_leaves_with_candidate_count` directly.
#[cfg(test)]
pub(crate) fn execute_classified_retained_leaves_with_options(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaves: &[RetainedLeaf],
    options: QueryExecutionOptions,
) -> RetainedLeafBatchExecutionReport {
    let candidate_count = classified_retained_candidate_count(index, retained_leaves);

    execute_classified_retained_leaves_with_candidate_count(
        index,
        query,
        retained_leaves,
        candidate_count,
        options,
    )
}

/// Executes classified retained leaves using an already known candidate count.
///
/// # Runtime Role
///
/// Traversal already knows how many records are contained by retained leaves.
/// This helper preserves the result-capacity advantage without requiring a
/// second pass over retained leaves before execution.
pub(crate) fn execute_classified_retained_leaves_with_candidate_count(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaves: &[RetainedLeaf],
    candidate_count: usize,
    options: QueryExecutionOptions,
) -> RetainedLeafBatchExecutionReport {
    match options.mode {
        QueryExecutionMode::Serial => {
            execute_classified_retained_leaves_serial_with_candidate_count(
                index,
                query,
                retained_leaves,
                candidate_count,
            )
        }
        QueryExecutionMode::Parallel => {
            if should_execute_retained_leaves_in_parallel(options, retained_leaves.len()) {
                execute_classified_retained_leaves_parallel_with_candidate_count(
                    index,
                    query,
                    retained_leaves,
                    candidate_count,
                )
            } else {
                // rayon is not free
                execute_classified_retained_leaves_serial_with_candidate_count(
                    index,
                    query,
                    retained_leaves,
                    candidate_count,
                )
            }
        }
    }
}

/// Converts retained leaf identifiers into traversal-style retained leaf records.
///
/// # Runtime Role
///
/// This helper keeps older internal tests and helpers working while allowing
/// normal query execution to consume classified traversal output directly.
#[cfg(test)]
pub(crate) fn classify_retained_leaf_ids(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaf_ids: &[usize],
) -> Vec<RetainedLeaf> {
    validate_retained_leaf_ids(index, retained_leaf_ids);

    retained_leaf_ids
        .iter()
        .map(|node_id| {
            let node = &index.nodes[*node_id];

            if query.contains_bounds(&node.bounds) {
                RetainedLeaf::covered(*node_id)
            } else {
                RetainedLeaf::partial(*node_id)
            }
        })
        .collect()
}

/// Validates that retained node identifiers reference leaf partitions.
///
/// # Runtime Role
///
/// Retained leaf execution assumes Stage I traversal already selected valid
/// leaf partitions. This helper makes that boundary explicit so later execution
/// strategies can focus on reconstruction and evaluation instead of defensive
/// index checks.
///
/// # Panics
///
/// Panics when a retained node identifier is outside the index or references an
/// internal partition.
#[cfg(test)]
pub(crate) fn validate_retained_leaf_ids(index: &FSEIndex, retained_leaf_ids: &[usize]) {
    // dont let later paralel work inherit sketchy ids
    for node_id in retained_leaf_ids {
        let Some(node) = index.nodes.get(*node_id) else {
            panic!("retained leaf id {node_id} is outside index node range");
        };

        assert!(
            node.is_leaf,
            "retained leaf id {node_id} must reference a leaf partition"
        );
    }
}

/// Validates that retained leaf records reference leaf partitions.
///
/// # Runtime Role
///
/// Classified traversal output should already be valid, but this keeps execution
/// helpers safe when tests or future callers construct retained leaves directly.
#[cfg(any(test, debug_assertions))]
pub(crate) fn validate_retained_leaves(index: &FSEIndex, retained_leaves: &[RetainedLeaf]) {
    for retained_leaf in retained_leaves {
        let Some(node) = index.nodes.get(retained_leaf.node_id) else {
            panic!(
                "retained leaf id {} is outside index node range",
                retained_leaf.node_id
            );
        };

        assert!(
            node.is_leaf,
            "retained leaf id {} must reference a leaf partition",
            retained_leaf.node_id
        );
    }
}

/// Executes Stage II and Stage III for one retained leaf partition.
///
/// # Runtime Role
///
/// This compatibility helper classifies the retained leaf locally. Normal query
/// execution should prefer traversal-provided classification.
///
/// # Panics
///
/// Panics when `node` is not a leaf partition.
#[cfg(test)]
pub(crate) fn execute_retained_leaf(
    node: &PartitionNode,
    query: &QueryRegion,
    dimensions: usize,
) -> RetainedLeafExecutionReport {
    let coverage = if query.contains_bounds(&node.bounds) {
        RetainedLeafCoverage::Covered
    } else {
        RetainedLeafCoverage::Partial
    };

    execute_retained_leaf_with_coverage(node, query, dimensions, coverage)
}

/// Executes Stage II and Stage III for one retained leaf with known coverage.
///
/// # Runtime Role
///
/// The coverage classification comes from traversal in the normal query path.
/// Covered leaves skip exact per-row predicate checks. Partial leaves preserve
/// the exact predicate path.
///
/// This compatibility helper validates shape locally because it receives only a
/// node reference. The normal index execution path should use
/// [`execute_retained_leaf_with_cached_shape`].
#[cfg(test)]
pub(crate) fn execute_retained_leaf_with_coverage(
    node: &PartitionNode,
    query: &QueryRegion,
    dimensions: usize,
    coverage: RetainedLeafCoverage,
) -> RetainedLeafExecutionReport {
    let shape = leaf_reconstruction_shape_for_direct_node(node, dimensions);

    execute_retained_leaf_with_cached_shape(node, query, shape, coverage)
}

/// Executes one retained leaf using cached reconstruction shape metadata.
///
/// # Runtime Role
///
/// This is the leaf-local execution path used by parallel retained-leaf
/// execution. The index already validated and cached the leaf shape, so this
/// function avoids per-query shape validation.
pub(crate) fn execute_retained_leaf_with_cached_shape(
    node: &PartitionNode,
    query: &QueryRegion,
    shape: LeafReconstructionShape,
    coverage: RetainedLeafCoverage,
) -> RetainedLeafExecutionReport {
    assert!(
        node.is_leaf,
        "retained leaf execution helper requires a leaf node"
    );

    let mut batch_report =
        RetainedLeafBatchExecutionReport::with_candidate_capacity(shape.cardinality);

    match coverage {
        RetainedLeafCoverage::Covered => {
            append_covered_retained_leaf_results(node, shape, &mut batch_report);
        }
        RetainedLeafCoverage::Partial => {
            let mut reconstructed_values = Vec::with_capacity(shape.dimensions);
            append_partially_covered_retained_leaf_results(
                node,
                shape,
                query,
                &mut batch_report,
                &mut reconstructed_values,
            );
        }
    }

    RetainedLeafExecutionReport {
        results: batch_report.results,
        reconstructed_records: batch_report.reconstructed_records,
        #[cfg(test)]
        predicate_evaluated_records: batch_report.predicate_evaluated_records,
        matched_records: batch_report.matched_records,
    }
}

/// Streams one retained leaf into an existing batch report.
///
/// # Runtime Role
///
/// This is the serial execution hot path. It preserves retained-leaf ordering
/// while avoiding a temporary result vector and merge step for each leaf.
pub(crate) fn execute_retained_leaf_into_batch_report(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
    coverage: RetainedLeafCoverage,
    batch_report: &mut RetainedLeafBatchExecutionReport,
    reconstructed_values: &mut Vec<Scalar>,
) {
    assert!(
        node.is_leaf,
        "retained leaf streaming helper requires a leaf node"
    );

    match coverage {
        RetainedLeafCoverage::Covered => {
            append_covered_retained_leaf_results(node, shape, batch_report)
        }
        RetainedLeafCoverage::Partial => append_partially_covered_retained_leaf_results(
            node,
            shape,
            query,
            batch_report,
            reconstructed_values,
        ),
    }
}

/// Appends all rows from a covered retained leaf into the batch report.
///
/// # Runtime Role
///
/// The query already contains the leaf bounds, so every reconstructed row can be
/// appended directly without exact predicate evaluation.
pub(crate) fn append_covered_retained_leaf_results(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    batch_report: &mut RetainedLeafBatchExecutionReport,
) {
    debug_assert_eq!(
        node.dimensions(),
        shape.dimensions,
        "cached leaf dimensionality should match node dimensionality"
    );
    debug_assert_eq!(
        node.residuals.cardinality(),
        shape.cardinality,
        "cached leaf cardinality should match residual cardinality"
    );

    reserve_additional_results(&mut batch_report.results, shape.cardinality);

    // geometry already proved these rows match
    for row in 0..shape.cardinality {
        batch_report
            .results
            .push(reconstruct_point_prevalidated(node, row, shape.dimensions));
    }

    batch_report.reconstructed_records += shape.cardinality;
    batch_report.matched_records += shape.cardinality;
}

/// Appends matching rows from a partially covered retained leaf into the batch report.
///
/// # Runtime Role
///
/// Partial leaves still use the exact predicate path. The retained-leaf shape is
/// read from the index cache before the row loop, then each candidate row is
/// reconstructed into the reusable scratch buffer before exact query evaluation.
pub(crate) fn append_partially_covered_retained_leaf_results(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    query: &QueryRegion,
    batch_report: &mut RetainedLeafBatchExecutionReport,
    reconstructed_values: &mut Vec<Scalar>,
) {
    debug_assert_eq!(
        node.dimensions(),
        shape.dimensions,
        "cached leaf dimensionality should match node dimensionality"
    );
    debug_assert_eq!(
        node.residuals.cardinality(),
        shape.cardinality,
        "cached leaf cardinality should match residual cardinality"
    );

    let original_match_count = batch_report.results.len();

    // two-pass row handling was faster for the tiny 2d benchmark than fusion
    for row in 0..shape.cardinality {
        reconstruct_row_into_prevalidated(node, row, shape.dimensions, reconstructed_values);

        if query.contains_values(reconstructed_values) {
            push_reconstructed_values_as_result(
                &mut batch_report.results,
                reconstructed_values,
                shape.dimensions,
            );
        }
    }

    let matched_records = batch_report.results.len() - original_match_count;

    batch_report.reconstructed_records += shape.cardinality;
    #[cfg(test)]
    {
        batch_report.predicate_evaluated_records += shape.cardinality;
    }
    batch_report.matched_records += matched_records;
}

/// Moves a reconstructed row buffer into the final result set.
///
/// # Runtime Role
///
/// Matching rows must still become owned `Vector` values because that is the
/// query API contract. Moving the scratch buffer avoids cloning the same row
/// after exact predicate evaluation has already accepted it.
fn push_reconstructed_values_as_result(
    results: &mut Vec<Vector>,
    reconstructed_values: &mut Vec<Scalar>,
    dimensions: usize,
) {
    debug_assert_eq!(
        reconstructed_values.len(),
        dimensions,
        "reconstructed row dimensionality should match the partition"
    );

    let accepted_values = std::mem::replace(reconstructed_values, Vec::with_capacity(dimensions));

    results.push(Vector::new(accepted_values));
}

/// Executes a retained leaf whose bounding box is fully contained by the query.
///
/// # Runtime Role
///
/// Covered leaves skip exact per-row predicate checks. Reconstruction still
/// happens because query output is expressed as coordinate vectors, but every
/// reconstructed row can be appended directly to the result set.
#[cfg(test)]
pub(crate) fn execute_covered_retained_leaf(
    node: &PartitionNode,
    dimensions: usize,
) -> RetainedLeafExecutionReport {
    assert!(
        node.is_leaf,
        "covered retained leaf helper requires a leaf node"
    );

    let shape = leaf_reconstruction_shape_for_direct_node(node, dimensions);
    let mut batch_report =
        RetainedLeafBatchExecutionReport::with_candidate_capacity(shape.cardinality);

    append_covered_retained_leaf_results(node, shape, &mut batch_report);

    RetainedLeafExecutionReport {
        results: batch_report.results,
        reconstructed_records: batch_report.reconstructed_records,
        #[cfg(test)]
        predicate_evaluated_records: batch_report.predicate_evaluated_records,
        matched_records: batch_report.matched_records,
    }
}

/// Executes a retained leaf whose bounding box only partially overlaps the query.
///
/// # Runtime Role
///
/// Partially covered leaves preserve the full exact predicate path. Each
/// candidate row reconstructs coordinate values and checks them against the
/// query before becoming an owned `Vector`.
#[cfg(test)]
pub(crate) fn execute_partially_covered_retained_leaf(
    node: &PartitionNode,
    query: &QueryRegion,
    dimensions: usize,
) -> RetainedLeafExecutionReport {
    assert!(
        node.is_leaf,
        "partial retained leaf helper requires a leaf node"
    );

    let shape = leaf_reconstruction_shape_for_direct_node(node, dimensions);
    let mut batch_report =
        RetainedLeafBatchExecutionReport::with_candidate_capacity(shape.cardinality);
    let mut reconstructed_values = Vec::with_capacity(shape.dimensions);

    append_partially_covered_retained_leaf_results(
        node,
        shape,
        query,
        &mut batch_report,
        &mut reconstructed_values,
    );

    RetainedLeafExecutionReport {
        results: batch_report.results,
        reconstructed_records: batch_report.reconstructed_records,
        #[cfg(test)]
        predicate_evaluated_records: batch_report.predicate_evaluated_records,
        matched_records: batch_report.matched_records,
    }
}

/// Returns whether a query region fully contains a bounding box.
///
/// # Runtime Role
///
/// This predicate preserves the previous execution helper API while delegating
/// the actual containment logic to `QueryRegion`.
///
/// # Panics
///
/// Panics when dimensionality differs between the query and bounds.
#[cfg(test)]
pub(crate) fn query_contains_bounds(query: &QueryRegion, bounds: &BoundingBox) -> bool {
    assert_eq!(
        query.dimensions(),
        bounds.dimensions(),
        "query and bounds dimensionality must match"
    );

    query.contains_bounds(bounds)
}

/// Merges retained leaf reports in their supplied order.
///
/// # Runtime Role
///
/// This helper defines the deterministic merge contract for retained-leaf
/// execution. Parallel execution computes reports independently but still passes
/// them to this function in retained leaf order before final result assembly.
pub(crate) fn merge_retained_leaf_reports_in_order(
    leaf_reports: Vec<RetainedLeafExecutionReport>,
    candidate_count: usize,
) -> RetainedLeafBatchExecutionReport {
    let mut results = Vec::with_capacity(result_capacity_hint(candidate_count));
    let mut aggregate_stats = QueryExecutionStats::default();

    #[cfg(test)]
    let mut predicate_evaluated_records = 0;

    // parallel reports still merge here
    for leaf_report in leaf_reports {
        #[cfg(test)]
        {
            predicate_evaluated_records += leaf_report.predicate_evaluated_records;
        }

        merge_retained_leaf_report(&mut results, &mut aggregate_stats, leaf_report);
    }

    RetainedLeafBatchExecutionReport {
        results,
        reconstructed_records: aggregate_stats.reconstructed_records,
        #[cfg(test)]
        predicate_evaluated_records,
        matched_records: aggregate_stats.matched_records,
    }
}

/// Merges one retained leaf report into the final query result.
///
/// # Runtime Role
///
/// This helper keeps result merging and execution-stat aggregation in one place
/// for the parallel path, where leaf-local result vectors are still required.
pub(crate) fn merge_retained_leaf_report(
    results: &mut Vec<Vector>,
    stats: &mut QueryExecutionStats,
    leaf_report: RetainedLeafExecutionReport,
) {
    let incoming_results = leaf_report.results.len();

    stats.reconstructed_records += leaf_report.reconstructed_records;
    stats.matched_records += leaf_report.matched_records;

    // merge step gets its own small seam now
    reserve_additional_results(results, incoming_results);
    results.extend(leaf_report.results);
}

/// Reserves enough final result capacity for an incoming result batch.
///
/// # Runtime Role
///
/// The final query result vector may start with a bounded capacity hint. If
/// actual matches exceed that initial hint, this helper reserves exactly the
/// additional space needed before appending more results.
pub(crate) fn reserve_additional_results(results: &mut Vec<Vector>, incoming_len: usize) {
    let available_capacity = results.capacity().saturating_sub(results.len());

    if incoming_len > available_capacity {
        // just enough room for this batch
        results.reserve_exact(incoming_len - available_capacity);
    }
}

/// Returns the number of records contained in retained leaves.
///
/// # Runtime Role
///
/// This count is the maximum number of rows that can be returned by exact
/// evaluation after geometric pruning. It is also the number of rows that Stage
/// II will reconstruct.
///
/// # Panics
///
/// Panics if any retained node identifier is outside the index node range.
#[cfg(test)]
pub(crate) fn retained_candidate_count(index: &FSEIndex, retained_leaf_ids: &[usize]) -> usize {
    retained_leaf_ids
        .iter()
        .map(|node_id| index.nodes[*node_id].residuals.cardinality())
        .sum()
}

/// Returns the number of records contained in classified retained leaves.
///
/// # Runtime Role
///
/// This is the classified retained-leaf equivalent of `retained_candidate_count`.
#[cfg(test)]
pub(crate) fn classified_retained_candidate_count(
    index: &FSEIndex,
    retained_leaves: &[RetainedLeaf],
) -> usize {
    retained_leaves
        .iter()
        .map(|retained_leaf| index.nodes[retained_leaf.node_id].residuals.cardinality())
        .sum()
}

/// Builds a local reconstruction shape for helpers that receive only a node.
///
/// # Runtime Role
///
/// Normal index execution uses cached shape metadata from `FSEIndex`. This
/// helper exists for test-facing functions that operate directly on a
/// `PartitionNode`.
#[cfg(test)]
fn leaf_reconstruction_shape_for_direct_node(
    node: &PartitionNode,
    dimensions: usize,
) -> LeafReconstructionShape {
    assert!(node.is_leaf, "retained leaf helper requires a leaf node");

    let validated_shape = validate_partition_reconstruction_shape(node);

    assert_eq!(
        validated_shape.dimensions, dimensions,
        "provided dimensions must match leaf dimensionality"
    );

    LeafReconstructionShape::new(
        node.id,
        validated_shape.dimensions,
        validated_shape.cardinality,
    )
}
