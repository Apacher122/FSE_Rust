//! Query reference-result tests.

use crate::benchmark::{
    QueryWorkloadCase, clustered_points_2d, clustered_workload_cases, flat_scan,
    large_clustered_points_2d, large_clustered_workload_cases,
};
use crate::build::{BuildConfig, FSEBuilder};
use crate::math::Vector;
use crate::query::{
    QueryResultReference, count_query_matches_with_stats, execute_query_references_with_stats,
    reconstruct_query_result_reference, reconstruct_query_result_reference_into,
    reconstruct_query_result_references, reconstruct_query_result_references_into,
};
use crate::storage::FSEIndex;
use crate::tests::support::sort_points;

fn reconstruct_references(index: &FSEIndex, references: &[QueryResultReference]) -> Vec<Vector> {
    reconstruct_query_result_references(index, references)
}

fn assert_reference_query_matches_flat_scan_for_workloads(
    points: &[Vector],
    max_depth: usize,
    workloads: &[QueryWorkloadCase],
) {
    let builder = FSEBuilder::new(BuildConfig::new(8, max_depth).with_target_leaf_size(8));
    let index = builder.build(points);

    for workload in workloads {
        let reference_report = execute_query_references_with_stats(&index, &workload.query);
        let count_report = count_query_matches_with_stats(&index, &workload.query);

        let mut reconstructed_matches = reconstruct_references(&index, &reference_report.matches);
        let mut expected_matches = flat_scan(points, &workload.query);

        sort_points(&mut reconstructed_matches);
        sort_points(&mut expected_matches);

        assert_eq!(
            reconstructed_matches, expected_matches,
            "reference query reconstructed matches differed from flat scan for workload `{}`",
            workload.name
        );

        assert_eq!(
            reference_report.matches.len(),
            expected_matches.len(),
            "reference query match count differed from flat scan for workload `{}`",
            workload.name
        );

        assert_eq!(
            reference_report.stats, count_report.stats,
            "reference query stats differed from count-only stats for workload `{}`",
            workload.name
        );
    }
}

#[test]
fn reference_query_matches_flat_scan_for_small_benchmark_workloads() {
    let points = clustered_points_2d();
    let workloads = clustered_workload_cases();

    assert_reference_query_matches_flat_scan_for_workloads(&points, 8, &workloads);
}

#[test]
fn reference_query_matches_flat_scan_for_large_benchmark_workloads() {
    let points = large_clustered_points_2d();
    let workloads = large_clustered_workload_cases();

    assert_reference_query_matches_flat_scan_for_workloads(&points, 16, &workloads);
}

#[test]
fn reconstruct_query_result_reference_into_reuses_output_buffer() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8).with_target_leaf_size(8));
    let index = builder.build(&points);
    let workload = clustered_workload_cases()
        .into_iter()
        .find(|workload| workload.name == "cluster_boundary_range")
        .expect("small benchmark workloads should include cluster_boundary_range");

    let reference = execute_query_references_with_stats(&index, &workload.query)
        .matches
        .into_iter()
        .next()
        .expect("cluster_boundary_range should return at least one reference");

    let mut output = Vec::with_capacity(16);
    let original_capacity = output.capacity();

    reconstruct_query_result_reference_into(&index, reference, &mut output);

    let reconstructed = reconstruct_query_result_reference(&index, reference);

    assert_eq!(output, reconstructed.values);
    assert!(
        output.capacity() >= original_capacity,
        "reference reconstruction should preserve reusable output capacity"
    );
}

#[test]
fn reconstruct_query_result_references_matches_single_reference_reconstruction() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8).with_target_leaf_size(8));
    let index = builder.build(&points);
    let workload = clustered_workload_cases()
        .into_iter()
        .find(|workload| workload.name == "cluster_boundary_range")
        .expect("small benchmark workloads should include cluster_boundary_range");

    let reference_report = execute_query_references_with_stats(&index, &workload.query);

    let batch_results = reconstruct_query_result_references(&index, &reference_report.matches);
    let single_results: Vec<Vector> = reference_report
        .matches
        .iter()
        .map(|reference| reconstruct_query_result_reference(&index, *reference))
        .collect();

    assert_eq!(
        batch_results, single_results,
        "batch reference reconstruction should match single-reference reconstruction"
    );
}

#[test]
fn reconstruct_query_result_references_into_reuses_inner_coordinate_buffers() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8).with_target_leaf_size(8));
    let index = builder.build(&points);
    let workload = clustered_workload_cases()
        .into_iter()
        .find(|workload| workload.name == "cluster_boundary_range")
        .expect("small benchmark workloads should include cluster_boundary_range");

    let reference_report = execute_query_references_with_stats(&index, &workload.query);

    assert!(
        !reference_report.matches.is_empty(),
        "test setup should produce reference rows"
    );

    let mut reusable_results =
        reconstruct_query_result_references(&index, &reference_report.matches);

    for result in &mut reusable_results {
        result.values.reserve_exact(8);
    }

    let expected_results = reusable_results.clone();
    let original_value_pointers: Vec<*const crate::math::Scalar> = reusable_results
        .iter()
        .map(|result| result.values.as_ptr())
        .collect();
    let original_value_capacities: Vec<usize> = reusable_results
        .iter()
        .map(|result| result.values.capacity())
        .collect();

    reconstruct_query_result_references_into(
        &index,
        &reference_report.matches,
        &mut reusable_results,
    );

    let reused_value_pointers: Vec<*const crate::math::Scalar> = reusable_results
        .iter()
        .map(|result| result.values.as_ptr())
        .collect();
    let reused_value_capacities: Vec<usize> = reusable_results
        .iter()
        .map(|result| result.values.capacity())
        .collect();

    assert_eq!(reusable_results, expected_results);
    assert_eq!(
        reused_value_pointers, original_value_pointers,
        "batch reference reconstruction should reuse inner coordinate buffers"
    );
    assert_eq!(
        reused_value_capacities, original_value_capacities,
        "batch reference reconstruction should preserve reusable inner buffer capacity"
    );
}

#[test]
fn reconstruct_query_result_references_into_truncates_stale_rows() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8).with_target_leaf_size(8));
    let index = builder.build(&points);
    let workload = clustered_workload_cases()
        .into_iter()
        .find(|workload| workload.name == "cluster_boundary_range")
        .expect("small benchmark workloads should include cluster_boundary_range");

    let reference_report = execute_query_references_with_stats(&index, &workload.query);
    let first_reference = reference_report
        .matches
        .first()
        .copied()
        .expect("cluster_boundary_range should return at least one reference");

    let mut reusable_results =
        reconstruct_query_result_references(&index, &reference_report.matches);

    assert!(
        reusable_results.len() > 1,
        "test setup should start with multiple reconstructed rows"
    );

    reconstruct_query_result_references_into(&index, &[first_reference], &mut reusable_results);

    assert_eq!(reusable_results.len(), 1);
    assert_eq!(
        reusable_results[0],
        reconstruct_query_result_reference(&index, first_reference)
    );
}

#[test]
#[should_panic(expected = "must reference a leaf partition")]
fn reconstruct_query_result_reference_rejects_internal_node_reference() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8).with_target_leaf_size(8));
    let index = builder.build(&points);

    assert!(
        !index.nodes[index.root].is_leaf,
        "test setup should build a multi-node index"
    );

    reconstruct_query_result_reference(
        &index,
        QueryResultReference {
            node_id: index.root,
            row_index: 0,
        },
    );
}

#[test]
#[should_panic(expected = "residual row index must be inside")]
fn reconstruct_query_result_reference_rejects_out_of_range_row() {
    let points = clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 8).with_target_leaf_size(8));
    let index = builder.build(&points);
    let leaf_node_id = *index
        .leaf_node_ids()
        .first()
        .expect("test setup should include at least one leaf");
    let shape = index.leaf_reconstruction_shape(leaf_node_id);

    reconstruct_query_result_reference(
        &index,
        QueryResultReference {
            node_id: leaf_node_id,
            row_index: shape.cardinality,
        },
    );
}
