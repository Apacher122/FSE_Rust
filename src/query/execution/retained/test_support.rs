//! Test-only retained-leaf compatibility helpers.

use crate::math::BoundingBox;
use crate::query::reconstruction::validate_partition_reconstruction_shape;
use crate::query::{QueryRegion, RetainedLeaf, RetainedLeafCoverage};
use crate::storage::{FSEIndex, LeafReconstructionShape, PartitionNode};

use super::super::options::QueryExecutionOptions;
use super::super::reports::{RetainedLeafBatchExecutionReport, RetainedLeafExecutionReport};
use super::dispatch::execute_classified_retained_leaves_with_candidate_count;
use super::leaf::{
    append_covered_retained_leaf_results, append_partially_covered_retained_leaf_results,
    execute_retained_leaf_with_cached_shape,
};

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
pub(crate) fn execute_retained_leaves_serial(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaf_ids: &[usize],
) -> RetainedLeafBatchExecutionReport {
    let retained_leaves = classify_retained_leaf_ids(index, query, retained_leaf_ids);

    super::super::serial::execute_classified_retained_leaves_serial(index, query, &retained_leaves)
}

/// Executes retained leaf identifiers using Rayon-backed parallel iteration.
///
/// # Runtime Role
///
/// This preserves the older test/helper API by classifying leaf ids before
/// dispatching to the classified retained-leaf execution path.
pub(crate) fn execute_retained_leaves_parallel(
    index: &FSEIndex,
    query: &QueryRegion,
    retained_leaf_ids: &[usize],
) -> RetainedLeafBatchExecutionReport {
    let retained_leaves = classify_retained_leaf_ids(index, query, retained_leaf_ids);

    super::super::parallel::execute_classified_retained_leaves_parallel(
        index,
        query,
        &retained_leaves,
    )
}

/// Executes already classified retained leaves using explicit options.
///
/// # Runtime Role
///
/// This compatibility helper is test-only. The release query path supplies the
/// retained candidate count from traversal and uses
/// `execute_classified_retained_leaves_with_candidate_count` directly.
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

/// Converts retained leaf identifiers into traversal-style retained leaf records.
///
/// # Runtime Role
///
/// This helper keeps older internal tests and helpers working while allowing
/// normal query execution to consume classified traversal output directly.
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
            let shape = index.leaf_reconstruction_shape(*node_id);

            if query.contains_bounds(&node.bounds) {
                RetainedLeaf::covered_with_shape(shape)
            } else {
                RetainedLeaf::partial_with_shape(shape)
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
/// `execute_retained_leaf_with_cached_shape`.
pub(crate) fn execute_retained_leaf_with_coverage(
    node: &PartitionNode,
    query: &QueryRegion,
    dimensions: usize,
    coverage: RetainedLeafCoverage,
) -> RetainedLeafExecutionReport {
    let shape = leaf_reconstruction_shape_for_direct_node(node, dimensions);

    execute_retained_leaf_with_cached_shape(node, query, shape, coverage)
}

/// Executes a retained leaf whose bounding box is fully contained by the query.
///
/// # Runtime Role
///
/// Covered leaves skip exact per-row predicate checks. Reconstruction still
/// happens because query output is expressed as coordinate vectors, but every
/// reconstructed row can be appended directly to the result set.
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
pub(crate) fn query_contains_bounds(query: &QueryRegion, bounds: &BoundingBox) -> bool {
    assert_eq!(
        query.dimensions(),
        bounds.dimensions(),
        "query and bounds dimensionality must match"
    );

    query.contains_bounds(bounds)
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
