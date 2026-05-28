//! Guarded structural gap split helpers.

use std::cmp::Ordering;

use crate::math::{Scalar, Vector};

const STRUCTURAL_GAP_DOMINANCE_RATIO: Scalar = 4.0;

pub(super) fn guarded_structural_split_index(sorted: &[Vector], split_dimension: usize) -> usize {
    structural_gap_split_index(sorted, split_dimension).unwrap_or(sorted.len() / 2)
}

fn structural_gap_split_index(sorted: &[Vector], split_dimension: usize) -> Option<usize> {
    debug_assert!(
        sorted.len() >= 2,
        "structural gap split requires at least two sorted points"
    );

    let median_index = sorted.len() / 2;
    let mut best_split_index = median_index;
    let mut best_gap = Scalar::NEG_INFINITY;
    let mut best_median_distance = usize::MAX;
    let mut positive_gaps = Vec::with_capacity(sorted.len().saturating_sub(1));

    for split_index in 1..sorted.len() {
        let previous_value = sorted[split_index - 1].values[split_dimension];
        let next_value = sorted[split_index].values[split_dimension];
        let gap = next_value - previous_value;

        if gap > 0.0 {
            positive_gaps.push(gap);
        }

        let median_distance = split_index.abs_diff(median_index);

        if gap > best_gap || (gap == best_gap && median_distance < best_median_distance) {
            best_gap = gap;
            best_split_index = split_index;
            best_median_distance = median_distance;
        }
    }

    if !gap_is_structural(best_gap, &mut positive_gaps) {
        return None;
    }

    Some(best_split_index)
}

fn gap_is_structural(largest_gap: Scalar, positive_gaps: &mut Vec<Scalar>) -> bool {
    if largest_gap <= 0.0 || positive_gaps.is_empty() {
        return false;
    }

    if positive_gaps.len() == 1 {
        return true;
    }

    positive_gaps.sort_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal));

    let local_spacing = lower_median_positive_gap(positive_gaps);

    if local_spacing <= 0.0 {
        return true;
    }

    // largest gap has to stand out from normal local spacing
    largest_gap >= local_spacing * STRUCTURAL_GAP_DOMINANCE_RATIO
}

fn lower_median_positive_gap(sorted_positive_gaps: &[Scalar]) -> Scalar {
    let median_index = (sorted_positive_gaps.len() - 1) / 2;

    sorted_positive_gaps[median_index]
}
