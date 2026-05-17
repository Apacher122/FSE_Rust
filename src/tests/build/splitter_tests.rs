use crate::build::splitter::{
    best_median_split_axis_score, median_split, median_split_on_axis, median_split_score_on_axis,
    select_split_axis,
};
use crate::build::variance::max_variance_dimension;
use crate::math::Vector;

#[test]
fn select_split_axis_chooses_dimension_with_highest_variance_when_volume_ties() {
    let points = vec![
        Vector::new(vec![0.0, 1.0]),
        Vector::new(vec![10.0, 2.0]),
        Vector::new(vec![20.0, 3.0]),
    ];

    let axis = select_split_axis(&points);

    assert_eq!(axis, 0);
}

#[test]
fn select_split_axis_prefers_lower_child_bounding_volume_over_raw_variance() {
    let points = volume_sensitive_points();

    assert_eq!(max_variance_dimension(&points), 0);

    let axis = select_split_axis(&points);

    assert_eq!(axis, 1);
}

#[test]
fn best_median_split_axis_score_reports_volume_optimized_axis() {
    let points = volume_sensitive_points();

    let score = best_median_split_axis_score(&points);

    assert_eq!(score.split_dimension, 1);
    assert_eq!(score.metrics.combined_child_volume, 1.0);
    assert_eq!(score.metrics.combined_child_extent, 12.0);
    assert_eq!(score.metrics.balance_penalty, 0);
    assert_eq!(score.combined_child_volume(), 1.0);
    assert_eq!(score.combined_child_extent(), 12.0);
    assert_eq!(score.balance_penalty(), 0);
}

#[test]
fn best_median_split_axis_score_uses_shared_split_quality_metrics() {
    let points = volume_sensitive_points();

    let score = best_median_split_axis_score(&points);
    let axis_score = median_split_score_on_axis(&points, score.split_dimension);

    assert_eq!(score.metrics, axis_score.metrics);
    assert_eq!(
        score.combined_child_volume(),
        axis_score.combined_child_volume()
    );
    assert_eq!(
        score.volume_reduction_ratio(),
        axis_score.volume_reduction_ratio()
    );
    assert_eq!(
        score.combined_child_extent(),
        axis_score.combined_child_extent()
    );
    assert_eq!(
        score.extent_reduction_ratio(),
        axis_score.extent_reduction_ratio()
    );
    assert_eq!(score.balance_penalty(), axis_score.balance_penalty());
}

#[test]
fn median_split_score_on_axis_reports_expected_volume_difference() {
    let points = volume_sensitive_points();

    let max_variance_axis_score = median_split_score_on_axis(&points, 0);
    let volume_optimized_axis_score = median_split_score_on_axis(&points, 1);

    assert_eq!(max_variance_axis_score.split_dimension, 0);
    assert_eq!(volume_optimized_axis_score.split_dimension, 1);
    assert!(
        volume_optimized_axis_score.combined_child_volume()
            < max_variance_axis_score.combined_child_volume()
    );
    assert!(
        volume_optimized_axis_score.volume_reduction_ratio()
            > max_variance_axis_score.volume_reduction_ratio()
    );
}

#[test]
fn median_split_score_on_axis_exposes_balance_penalty_from_metrics() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 0.0]),
        Vector::new(vec![2.0, 0.0]),
        Vector::new(vec![3.0, 0.0]),
        Vector::new(vec![4.0, 0.0]),
    ];

    let score = median_split_score_on_axis(&points, 0);

    assert_eq!(score.metrics.left_cardinality, 2);
    assert_eq!(score.metrics.right_cardinality, 3);
    assert_eq!(score.metrics.balance_penalty, 1);
    assert_eq!(score.balance_penalty(), 1);
}

#[test]
fn median_split_on_axis_splits_points_into_two_non_empty_groups() {
    let points = vec![
        Vector::new(vec![3.0, 0.0]),
        Vector::new(vec![1.0, 0.0]),
        Vector::new(vec![4.0, 0.0]),
        Vector::new(vec![2.0, 0.0]),
    ];

    let (left, right) = median_split_on_axis(&points, 0);

    assert_eq!(
        left,
        vec![Vector::new(vec![1.0, 0.0]), Vector::new(vec![2.0, 0.0]),]
    );

    assert_eq!(
        right,
        vec![Vector::new(vec![3.0, 0.0]), Vector::new(vec![4.0, 0.0]),]
    );
}

#[test]
fn median_split_uses_volume_optimized_axis() {
    let points = volume_sensitive_points();

    let (left, right) = median_split(&points);

    assert_eq!(
        left,
        vec![
            Vector::new(vec![0.0, 0.0]),
            Vector::new(vec![1.0, 0.0]),
            Vector::new(vec![10.0, 0.0]),
        ]
    );

    assert_eq!(
        right,
        vec![
            Vector::new(vec![0.0, 1.0]),
            Vector::new(vec![1.0, 1.0]),
            Vector::new(vec![0.0, 2.0]),
        ]
    );
}

#[test]
#[should_panic(expected = "median split requires at least two points")]
fn median_split_on_axis_rejects_single_point_input() {
    let points = vec![Vector::new(vec![0.0, 0.0])];

    let _ = median_split_on_axis(&points, 0);
}

#[test]
#[should_panic(expected = "split dimension must be inside point dimensionality")]
fn median_split_on_axis_rejects_out_of_range_dimension() {
    let points = vec![Vector::new(vec![0.0, 0.0]), Vector::new(vec![1.0, 1.0])];

    let _ = median_split_on_axis(&points, 2);
}

fn volume_sensitive_points() -> Vec<Vector> {
    vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![0.0, 1.0]),
        Vector::new(vec![0.0, 2.0]),
        Vector::new(vec![1.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![10.0, 0.0]),
    ]
}
