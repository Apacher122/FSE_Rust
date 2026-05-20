use crate::math::{BoundingBox, ResidualBlock, Vector};
use crate::query::execution::{
    execute_classified_retained_leaves_with_options, query_contains_bounds,
};
use crate::query::region::QueryBoundsClassification;
use crate::query::{
    QueryExecutionOptions, QueryRegion, RetainedLeaf, RetainedLeafCoverage, traverse_with_stats,
};
use crate::storage::{FSEIndex, PartitionNode};

#[test]
fn query_region_contains_bounds_when_box_is_inside_query() {
    let query = QueryRegion::new(vec![0.0, 0.0], vec![10.0, 10.0]);
    let bounds = BoundingBox::new(vec![2.0, 2.0], vec![8.0, 8.0]);

    assert!(query.contains_bounds(&bounds));
    assert!(query_contains_bounds(&query, &bounds));
}

#[test]
fn query_region_does_not_contain_bounds_when_box_escapes_query() {
    let query = QueryRegion::new(vec![0.0, 0.0], vec![10.0, 10.0]);
    let bounds = BoundingBox::new(vec![2.0, 2.0], vec![12.0, 8.0]);

    assert!(!query.contains_bounds(&bounds));
    assert!(!query_contains_bounds(&query, &bounds));
}

#[test]
fn query_region_intersects_bounds_when_box_overlaps_query() {
    let query = QueryRegion::new(vec![0.0, 0.0], vec![10.0, 10.0]);
    let bounds = BoundingBox::new(vec![8.0, 8.0], vec![12.0, 12.0]);

    assert!(query.intersects_bounds(&bounds));
}

#[test]
fn query_region_intersects_bounds_when_box_touches_query_boundary() {
    let query = QueryRegion::new(vec![0.0, 0.0], vec![10.0, 10.0]);
    let bounds = BoundingBox::new(vec![10.0, 3.0], vec![12.0, 5.0]);

    assert!(query.intersects_bounds(&bounds));
}

#[test]
fn query_region_does_not_intersect_bounds_when_box_is_separated() {
    let query = QueryRegion::new(vec![0.0, 0.0], vec![10.0, 10.0]);
    let bounds = BoundingBox::new(vec![11.0, 3.0], vec![12.0, 5.0]);

    assert!(!query.intersects_bounds(&bounds));
}

#[test]
fn query_region_does_not_intersect_bounds_when_dimensions_differ() {
    let query = QueryRegion::new(vec![0.0, 0.0], vec![10.0, 10.0]);
    let bounds = BoundingBox::new(vec![0.0], vec![10.0]);

    assert!(!query.intersects_bounds(&bounds));
}

#[test]
fn prevalidated_bounds_classification_matches_public_classification() {
    let query = QueryRegion::new(vec![0.0, 0.0], vec![10.0, 10.0]);

    let covered_bounds = BoundingBox::new(vec![2.0, 2.0], vec![8.0, 8.0]);
    let partial_bounds = BoundingBox::new(vec![8.0, 8.0], vec![12.0, 12.0]);
    let disjoint_bounds = BoundingBox::new(vec![11.0, 3.0], vec![12.0, 5.0]);

    // same answer just skipping the public dimension check
    assert_eq!(
        query.classify_bounds_prevalidated(&covered_bounds, 2),
        query.classify_bounds(&covered_bounds)
    );
    assert_eq!(
        query.classify_bounds_prevalidated(&partial_bounds, 2),
        query.classify_bounds(&partial_bounds)
    );
    assert_eq!(
        query.classify_bounds_prevalidated(&disjoint_bounds, 2),
        query.classify_bounds(&disjoint_bounds)
    );

    assert_eq!(
        query.classify_bounds_prevalidated(&covered_bounds, 2),
        QueryBoundsClassification::Covered
    );
    assert_eq!(
        query.classify_bounds_prevalidated(&partial_bounds, 2),
        QueryBoundsClassification::Partial
    );
    assert_eq!(
        query.classify_bounds_prevalidated(&disjoint_bounds, 2),
        QueryBoundsClassification::Disjoint
    );
}

#[test]
fn retained_leaf_helpers_report_coverage_kind() {
    let covered = RetainedLeaf::covered(4);
    let partial = RetainedLeaf::partial(7);

    assert_eq!(covered.node_id, 4);
    assert_eq!(covered.coverage, RetainedLeafCoverage::Covered);
    assert!(covered.is_covered());
    assert!(!covered.is_partial());

    assert_eq!(partial.node_id, 7);
    assert_eq!(partial.coverage, RetainedLeafCoverage::Partial);
    assert!(partial.is_partial());
    assert!(!partial.is_covered());
}

#[test]
fn traversal_classifies_single_leaf_as_covered_when_query_contains_bounds() {
    let points = vec![Vector::new(vec![1.0, 1.0]), Vector::new(vec![2.0, 2.0])];
    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);

    let query = QueryRegion::new(vec![0.0, 0.0], vec![3.0, 3.0]);
    let report = traverse_with_stats(&index, &query);

    assert_eq!(report.retained_leaf_ids, vec![0]);
    assert_eq!(report.retained_leaves, vec![RetainedLeaf::covered(0)]);
    assert_eq!(report.stats.visited_nodes, 1);
    assert_eq!(report.stats.retained_leaves, 1);
}

#[test]
fn traversal_classifies_single_leaf_as_partial_when_query_only_intersects_bounds() {
    let points = vec![Vector::new(vec![1.0, 1.0]), Vector::new(vec![4.0, 4.0])];
    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);

    let query = QueryRegion::new(vec![0.0, 0.0], vec![2.0, 2.0]);
    let report = traverse_with_stats(&index, &query);

    assert_eq!(report.retained_leaf_ids, vec![0]);
    assert_eq!(report.retained_leaves, vec![RetainedLeaf::partial(0)]);
    assert_eq!(report.stats.visited_nodes, 1);
    assert_eq!(report.stats.retained_leaves, 1);
}

#[test]
fn traversal_prunes_single_leaf_when_query_does_not_intersect_bounds() {
    let points = vec![Vector::new(vec![5.0, 5.0]), Vector::new(vec![6.0, 6.0])];
    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);

    let query = QueryRegion::new(vec![0.0, 0.0], vec![2.0, 2.0]);
    let report = traverse_with_stats(&index, &query);

    assert!(report.retained_leaf_ids.is_empty());
    assert!(report.retained_leaves.is_empty());
    assert_eq!(report.stats.visited_nodes, 1);
    assert_eq!(report.stats.retained_leaves, 0);
}

#[test]
fn traversal_classifies_descendant_leaves_as_covered_when_internal_subtree_is_covered() {
    let index = covered_subtree_index();

    let query = QueryRegion::new(vec![0.0, 0.0], vec![10.0, 10.0]);
    let report = traverse_with_stats(&index, &query);

    assert_eq!(report.retained_leaf_ids, vec![1, 2]);
    assert_eq!(
        report.retained_leaves,
        vec![RetainedLeaf::covered(1), RetainedLeaf::covered(2)]
    );
    assert_eq!(report.stats.visited_nodes, 3);
    assert_eq!(report.stats.total_leaves, 2);
    assert_eq!(report.stats.retained_leaves, 2);
    assert_eq!(report.stats.retained_leaf_ratio, 1.0);
}

#[test]
fn traversal_mixes_covered_and_partial_leaf_classification() {
    let index = covered_subtree_index();

    let query = QueryRegion::new(vec![0.0, 0.0], vec![5.0, 5.0]);
    let report = traverse_with_stats(&index, &query);

    assert_eq!(report.retained_leaf_ids, vec![1, 2]);
    assert_eq!(
        report.retained_leaves,
        vec![RetainedLeaf::covered(1), RetainedLeaf::partial(2)]
    );
    assert_eq!(report.stats.visited_nodes, 3);
    assert_eq!(report.stats.retained_leaves, 2);
}

#[test]
fn classified_retained_leaf_execution_uses_traversal_coverage() {
    let index = covered_subtree_index();

    let query = QueryRegion::new(vec![0.0, 0.0], vec![5.0, 5.0]);
    let traversal_report = traverse_with_stats(&index, &query);

    let execution_report = execute_classified_retained_leaves_with_options(
        &index,
        &query,
        &traversal_report.retained_leaves,
        QueryExecutionOptions::serial(),
    );

    assert_eq!(execution_report.reconstructed_records, 4);
    assert_eq!(execution_report.predicate_evaluated_records, 2);
    assert_eq!(execution_report.matched_records, 3);
    assert_eq!(
        execution_report.results,
        vec![
            Vector::new(vec![1.0, 1.0]),
            Vector::new(vec![2.0, 2.0]),
            Vector::new(vec![4.0, 4.0]),
        ]
    );
}

fn covered_subtree_index() -> FSEIndex {
    let root = PartitionNode::with_cardinality(
        0,
        vec![5.0, 5.0],
        BoundingBox::new(vec![0.0, 0.0], vec![10.0, 10.0]),
        ResidualBlock::new(vec![Vec::new(), Vec::new()]),
        4,
        vec![1, 2],
        false,
    );

    let covered_child = PartitionNode::from_points(
        1,
        &[Vector::new(vec![1.0, 1.0]), Vector::new(vec![2.0, 2.0])],
    );

    let partial_child = PartitionNode::from_points(
        2,
        &[Vector::new(vec![4.0, 4.0]), Vector::new(vec![8.0, 8.0])],
    );

    FSEIndex::new(vec![root, covered_child, partial_child], 0)
}
