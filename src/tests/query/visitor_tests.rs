//! Query reference visitor tests.

use crate::benchmark::{
    QueryWorkloadCase, clustered_points_2d, clustered_workload_cases, flat_scan,
    large_clustered_points_2d, large_clustered_workload_cases,
};
use crate::build::{BuildConfig, FSEBuilder};
use crate::math::{BoundingBox, ResidualBlock, Vector};
use crate::query::{
    QueryRegion, QueryResultReference, execute_query_references_with_stats,
    reconstruct_query_result_references, visit_query_references,
};
use crate::storage::{FSEIndex, PartitionNode};
use crate::tests::support::sort_points;

fn assert_visited_references_match_flat_scan_for_workloads(
    points: &[Vector],
    max_depth: usize,
    workloads: &[QueryWorkloadCase],
) {
    let builder = FSEBuilder::new(BuildConfig::new(8, max_depth).with_target_leaf_size(8));
    let index = builder.build(points);

    for workload in workloads {
        let mut visited_references = Vec::new();

        let visitor_stats = visit_query_references(&index, &workload.query, |reference| {
            visited_references.push(reference);
        });

        let reference_report = execute_query_references_with_stats(&index, &workload.query);

        assert_eq!(
            visited_references, reference_report.matches,
            "visitor references differed from reference-result API for workload `{}`",
            workload.name
        );

        assert_eq!(
            visitor_stats, reference_report.stats,
            "visitor stats differed from reference-result stats for workload `{}`",
            workload.name
        );

        let mut visited_matches = reconstruct_query_result_references(&index, &visited_references);
        let mut expected_matches = flat_scan(points, &workload.query);

        sort_points(&mut visited_matches);
        sort_points(&mut expected_matches);

        assert_eq!(
            visited_matches, expected_matches,
            "visitor reconstructed matches differed from flat scan for workload `{}`",
            workload.name
        );
    }
}

#[test]
fn visit_query_references_matches_reference_result_for_small_workloads() {
    let points = clustered_points_2d();
    let workloads = clustered_workload_cases();

    assert_visited_references_match_flat_scan_for_workloads(&points, 8, &workloads);
}

#[test]
fn visit_query_references_matches_reference_result_for_large_workloads() {
    let points = large_clustered_points_2d();
    let workloads = large_clustered_workload_cases();

    assert_visited_references_match_flat_scan_for_workloads(&points, 16, &workloads);
}

#[test]
fn visit_query_references_reports_root_disjoint_stats() {
    let points = vec![Vector::new(vec![0.0, 0.0]), Vector::new(vec![1.0, 1.0])];

    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);
    let query = QueryRegion::new(vec![10.0, 10.0], vec![20.0, 20.0]);

    let mut visited_references = Vec::<QueryResultReference>::new();
    let stats = visit_query_references(&index, &query, |reference| {
        visited_references.push(reference);
    });

    assert!(visited_references.is_empty());
    assert_eq!(stats.visited_nodes, 1);
    assert_eq!(stats.retained_leaves, 0);
    assert_eq!(stats.reconstructed_records, 0);
    assert_eq!(stats.matched_records, 0);
}

#[test]
fn visit_query_references_visits_every_root_covered_reference() {
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

    let mut visited_references = Vec::new();
    let stats = visit_query_references(&index, &query, |reference| {
        visited_references.push(reference);
    });

    assert_eq!(visited_references.len(), 4);
    assert_eq!(stats.reconstructed_records, 4);
    assert_eq!(stats.matched_records, 4);
}
