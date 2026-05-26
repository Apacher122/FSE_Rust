//! Retained candidate helpers for target workload diagnostics.

use super::super::context::BenchmarkApplicationContext;
use crate::math::{Scalar, Vector};
use crate::query::{QueryRegion, RetainedLeaf, RetainedLeafCoverage};
use crate::storage::{LeafReconstructionShape, PartitionNode};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct RetainedCandidateBreakdown {
    pub(super) covered_leaves: usize,
    pub(super) partial_leaves: usize,
    pub(super) covered_records: usize,
    pub(super) partial_records: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct RetainedCandidateRow {
    values: Vec<Scalar>,
    coverage: RetainedLeafCoverage,
}

pub(super) fn retained_candidate_breakdown(
    context: &BenchmarkApplicationContext,
    retained_leaves: &[RetainedLeaf],
) -> RetainedCandidateBreakdown {
    let mut breakdown = RetainedCandidateBreakdown::default();

    for retained_leaf in retained_leaves {
        let records = context.index.nodes[retained_leaf.node_id].stored_cardinality();

        match retained_leaf.coverage {
            RetainedLeafCoverage::Covered => {
                breakdown.covered_leaves += 1;
                breakdown.covered_records += records;
            }
            RetainedLeafCoverage::Partial => {
                breakdown.partial_leaves += 1;
                breakdown.partial_records += records;
            }
        }
    }

    breakdown
}

pub(super) fn reconstruct_retained_candidate_rows(
    context: &BenchmarkApplicationContext,
    retained_leaves: &[RetainedLeaf],
) -> Vec<RetainedCandidateRow> {
    let candidate_count = retained_leaves
        .iter()
        .map(|retained_leaf| context.index.nodes[retained_leaf.node_id].stored_cardinality())
        .sum();

    let mut rows = Vec::with_capacity(candidate_count);

    for retained_leaf in retained_leaves {
        let node = &context.index.nodes[retained_leaf.node_id];
        let shape = retained_leaf.reconstruction_shape(&context.index);

        append_reconstructed_candidate_rows(node, shape, retained_leaf.coverage, &mut rows);
    }

    rows
}

fn append_reconstructed_candidate_rows(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    coverage: RetainedLeafCoverage,
    rows: &mut Vec<RetainedCandidateRow>,
) {
    match shape.dimensions {
        1 => append_reconstructed_1d_candidate_rows(node, shape, coverage, rows),
        2 => append_reconstructed_2d_candidate_rows(node, shape, coverage, rows),
        _ => append_reconstructed_generic_candidate_rows(node, shape, coverage, rows),
    }
}

fn append_reconstructed_1d_candidate_rows(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    coverage: RetainedLeafCoverage,
    rows: &mut Vec<RetainedCandidateRow>,
) {
    let centroid_0 = node.centroid[0];
    let residual_0 = &node.residuals.dimensions[0];

    for row in 0..shape.cardinality {
        rows.push(RetainedCandidateRow {
            values: vec![centroid_0 + residual_0[row]],
            coverage,
        });
    }
}

fn append_reconstructed_2d_candidate_rows(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    coverage: RetainedLeafCoverage,
    rows: &mut Vec<RetainedCandidateRow>,
) {
    let centroid_0 = node.centroid[0];
    let centroid_1 = node.centroid[1];

    let residual_0 = &node.residuals.dimensions[0];
    let residual_1 = &node.residuals.dimensions[1];

    // this is diagnostic-only and intentionally mirrors the 2d retained path
    for row in 0..shape.cardinality {
        rows.push(RetainedCandidateRow {
            values: vec![centroid_0 + residual_0[row], centroid_1 + residual_1[row]],
            coverage,
        });
    }
}

fn append_reconstructed_generic_candidate_rows(
    node: &PartitionNode,
    shape: LeafReconstructionShape,
    coverage: RetainedLeafCoverage,
    rows: &mut Vec<RetainedCandidateRow>,
) {
    for row in 0..shape.cardinality {
        let mut values = Vec::with_capacity(shape.dimensions);

        for (centroid_value, residual_dimension) in
            node.centroid.iter().zip(&node.residuals.dimensions)
        {
            values.push(*centroid_value + residual_dimension[row]);
        }

        rows.push(RetainedCandidateRow { values, coverage });
    }
}

pub(super) fn count_matching_retained_candidate_rows(
    query: &QueryRegion,
    rows: &[RetainedCandidateRow],
) -> usize {
    rows.iter()
        .filter(|row| retained_candidate_row_matches(query, row))
        .count()
}

pub(super) fn matching_retained_candidate_values(
    query: &QueryRegion,
    rows: &[RetainedCandidateRow],
) -> Vec<Vec<Scalar>> {
    rows.iter()
        .filter(|row| retained_candidate_row_matches(query, row))
        .map(|row| row.values.clone())
        .collect()
}

fn retained_candidate_row_matches(query: &QueryRegion, row: &RetainedCandidateRow) -> bool {
    match row.coverage {
        RetainedLeafCoverage::Covered => true,
        RetainedLeafCoverage::Partial => {
            query.contains_values_prevalidated(&row.values, row.values.len())
        }
    }
}

pub(super) fn collect_matching_values_as_results(matched_values: &[Vec<Scalar>]) -> Vec<Vector> {
    let mut results = Vec::with_capacity(matched_values.len());

    for values in matched_values {
        results.push(Vector::new(values.clone()));
    }

    results
}
