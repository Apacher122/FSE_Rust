use crate::math::{BoundingBox, ResidualBlock, Vector};
use crate::query::execution::{
    RetainedLeafExecutionReport, execute_classified_retained_leaves_serial,
    execute_retained_leaves, merge_retained_leaf_reports_in_order, validate_retained_leaf_ids,
};
use crate::query::{QueryRegion, RetainedLeaf};
use crate::storage::{FSEIndex, PartitionNode};

#[test]
fn retained_leaf_batch_execution_returns_empty_report_when_no_leaves_are_retained() {
    let points = vec![Vector::new(vec![0.0, 0.0]), Vector::new(vec![1.0, 1.0])];

    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);
    let query = QueryRegion::new(vec![0.0, 0.0], vec![1.0, 1.0]);

    let report = execute_retained_leaves(&index, &query, &[]);

    assert!(report.results.is_empty());
    assert_eq!(report.reconstructed_records, 0);
    assert_eq!(report.predicate_evaluated_records, 0);
    assert_eq!(report.matched_records, 0);
}

#[test]
fn classified_serial_execution_streams_results_into_one_batch_report() {
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
    let query = QueryRegion::new(vec![0.0, 0.0], vec![8.0, 8.0]);
    let retained_leaves = vec![RetainedLeaf::covered(1), RetainedLeaf::partial(2)];

    let report = execute_classified_retained_leaves_serial(&index, &query, &retained_leaves);

    assert_eq!(
        report.results,
        vec![
            Vector::new(vec![0.0, 0.0]),
            Vector::new(vec![2.0, 2.0]),
            Vector::new(vec![8.0, 8.0]),
        ]
    );
    assert_eq!(report.reconstructed_records, 4);
    assert_eq!(report.predicate_evaluated_records, 2);
    assert_eq!(report.matched_records, 3);
}

#[test]
fn retained_leaf_batch_execution_merges_results_from_multiple_leaves() {
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
    let query = QueryRegion::new(vec![1.0, 1.0], vec![8.0, 8.0]);

    let report = execute_retained_leaves(&index, &query, &[1, 2]);

    assert_eq!(
        report.results,
        vec![Vector::new(vec![2.0, 2.0]), Vector::new(vec![8.0, 8.0])]
    );
    assert_eq!(report.reconstructed_records, 4);
    assert_eq!(report.predicate_evaluated_records, 4);
    assert_eq!(report.matched_records, 2);
}

#[test]
fn retained_leaf_batch_execution_preserves_supplied_retained_leaf_order() {
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
    let query = QueryRegion::new(vec![1.0, 1.0], vec![8.0, 8.0]);

    let report = execute_retained_leaves(&index, &query, &[2, 1]);

    assert_eq!(
        report.results,
        vec![Vector::new(vec![8.0, 8.0]), Vector::new(vec![2.0, 2.0])]
    );
    assert_eq!(report.reconstructed_records, 4);
    assert_eq!(report.predicate_evaluated_records, 4);
    assert_eq!(report.matched_records, 2);
}

#[test]
fn retained_leaf_batch_execution_filters_false_positive_retained_leaves() {
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
        &[Vector::new(vec![0.0, 0.0]), Vector::new(vec![10.0, 10.0])],
    );

    let right_child = PartitionNode::from_points(
        2,
        &[Vector::new(vec![20.0, 20.0]), Vector::new(vec![30.0, 30.0])],
    );

    let index = FSEIndex::new(vec![root, left_child, right_child], 0);
    let query = QueryRegion::new(vec![4.0, 4.0], vec![6.0, 6.0]);

    let report = execute_retained_leaves(&index, &query, &[1]);

    assert!(report.results.is_empty());
    assert_eq!(report.reconstructed_records, 2);
    assert_eq!(report.predicate_evaluated_records, 2);
    assert_eq!(report.matched_records, 0);
}

#[test]
fn retained_leaf_reports_merge_in_supplied_order() {
    let first_report = RetainedLeafExecutionReport {
        results: vec![Vector::new(vec![2.0, 2.0])],
        reconstructed_records: 2,
        predicate_evaluated_records: 2,
        matched_records: 1,
    };

    let second_report = RetainedLeafExecutionReport {
        results: vec![Vector::new(vec![8.0, 8.0]), Vector::new(vec![10.0, 10.0])],
        reconstructed_records: 2,
        predicate_evaluated_records: 0,
        matched_records: 2,
    };

    let report = merge_retained_leaf_reports_in_order(vec![second_report, first_report], 4);

    assert_eq!(
        report.results,
        vec![
            Vector::new(vec![8.0, 8.0]),
            Vector::new(vec![10.0, 10.0]),
            Vector::new(vec![2.0, 2.0]),
        ]
    );
    assert_eq!(report.reconstructed_records, 4);
    assert_eq!(report.predicate_evaluated_records, 2);
    assert_eq!(report.matched_records, 3);
}

#[test]
fn retained_leaf_reports_merge_empty_report_list() {
    let report = merge_retained_leaf_reports_in_order(Vec::new(), 0);

    assert!(report.results.is_empty());
    assert_eq!(report.reconstructed_records, 0);
    assert_eq!(report.predicate_evaluated_records, 0);
    assert_eq!(report.matched_records, 0);
}

#[test]
fn retained_leaf_id_validation_accepts_leaf_ids() {
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

    validate_retained_leaf_ids(&index, &[1, 2]);
}

#[test]
#[should_panic(expected = "retained leaf id 99 is outside index node range")]
fn retained_leaf_id_validation_rejects_out_of_range_ids() {
    let points = vec![Vector::new(vec![0.0, 0.0]), Vector::new(vec![1.0, 1.0])];
    let root = PartitionNode::from_points(0, &points);
    let index = FSEIndex::from_root(root);

    validate_retained_leaf_ids(&index, &[99]);
}

#[test]
#[should_panic(expected = "retained leaf id 0 must reference a leaf partition")]
fn retained_leaf_id_validation_rejects_internal_node_ids() {
    let root = PartitionNode::with_cardinality(
        0,
        vec![5.0, 5.0],
        BoundingBox::new(vec![0.0, 0.0], vec![10.0, 10.0]),
        ResidualBlock::new(vec![Vec::new(), Vec::new()]),
        2,
        vec![1],
        false,
    );

    let child = PartitionNode::from_points(
        1,
        &[Vector::new(vec![0.0, 0.0]), Vector::new(vec![2.0, 2.0])],
    );

    let index = FSEIndex::new(vec![root, child], 0);

    validate_retained_leaf_ids(&index, &[0]);
}
