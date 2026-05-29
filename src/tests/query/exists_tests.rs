//! Exact existence query tests.

use crate::benchmark::{
    QueryWorkloadCase, clustered_points_2d, clustered_workload_cases, large_clustered_points_2d,
    large_clustered_workload_cases,
};
use crate::build::{BuildConfig, FSEBuilder};
use crate::math::{BoundingBox, ResidualBlock, Vector};
use crate::query::{QueryRegion, count_query_matches, execute_query, query_has_match};
use crate::storage::{FSEIndex, PartitionNode};

#[test]
fn query_has_match_matches_owned_result_presence() {
    let points = vec![
        Vector::new(vec![0.0, 0.0]),
        Vector::new(vec![1.0, 1.0]),
        Vector::new(vec![2.0, 2.0]),
        Vector::new(vec![3.0, 3.0]),
    ];

    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);
    let query = QueryRegion::new(vec![1.0, 1.0], vec![2.0, 2.0]);

    assert!(query_has_match(&index, &query));
    assert_eq!(
        query_has_match(&index, &query),
        !execute_query(&index, &query).is_empty()
    );
}

#[test]
fn query_has_match_reports_false_for_root_disjoint_query() {
    let points = vec![Vector::new(vec![0.0, 0.0]), Vector::new(vec![1.0, 1.0])];

    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);
    let query = QueryRegion::new(vec![10.0, 10.0], vec![20.0, 20.0]);

    assert!(!query_has_match(&index, &query));
}

#[test]
fn query_has_match_uses_root_coverage_without_owned_results() {
    let root = PartitionNode::with_cardinality(
        0,
        vec![5.0, 5.0],
        BoundingBox::new(vec![0.0, 0.0], vec![10.0, 10.0]),
        ResidualBlock::new(vec![Vec::new(), Vec::new()]),
        4,
        vec![1, 2],
        false,
    );

    let left_child = PartitionNode::from_points(
        1,
        &[Vector::new(vec![0.0, 0.0]), Vector::new(vec![2.0, 2.0])],
    );

    let right_child = PartitionNode::from_points(
        2,
        &[Vector::new(vec![8.0, 8.0]), Vector::new(vec![10.0, 10.0])],
    );

    let index = FSEIndex::new(vec![root, left_child, right_child], 0);
    let query = QueryRegion::new(vec![-1.0, -1.0], vec![11.0, 11.0]);

    assert!(query_has_match(&index, &query));
}

#[test]
fn query_has_match_filters_false_positive_leaf_retention() {
    let points = vec![Vector::new(vec![0.0, 0.0]), Vector::new(vec![10.0, 10.0])];

    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);
    let query = QueryRegion::new(vec![4.0, 4.0], vec![6.0, 6.0]);

    assert!(!query_has_match(&index, &query));
    assert_eq!(count_query_matches(&index, &query), 0);
}

fn assert_exists_matches_count_only_for_benchmark_workloads(
    points: &[Vector],
    max_depth: usize,
    workloads: &[QueryWorkloadCase],
) {
    let builder = FSEBuilder::new(BuildConfig::new(8, max_depth).with_target_leaf_size(8));
    let index = builder.build(points);

    for workload in workloads {
        let has_match = query_has_match(&index, &workload.query);
        let count = count_query_matches(&index, &workload.query);

        assert_eq!(
            has_match,
            count > 0,
            "existence query result should match count-only result for workload `{}`",
            workload.name
        );
    }
}

#[test]
fn query_has_match_matches_count_only_for_small_benchmark_workloads() {
    let points = clustered_points_2d();
    let workloads = clustered_workload_cases();

    assert_exists_matches_count_only_for_benchmark_workloads(&points, 8, &workloads);
}

#[test]
fn query_has_match_matches_count_only_for_large_benchmark_workloads() {
    let points = large_clustered_points_2d();
    let workloads = large_clustered_workload_cases();

    assert_exists_matches_count_only_for_benchmark_workloads(&points, 16, &workloads);
}
