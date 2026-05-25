//! Query reference-result tests.

use crate::benchmark::{
    QueryWorkloadCase, clustered_points_2d, clustered_workload_cases, flat_scan,
    large_clustered_points_2d, large_clustered_workload_cases,
};
use crate::build::{BuildConfig, FSEBuilder};
use crate::math::Vector;
use crate::query::{
    QueryResultReference, count_query_matches_with_stats, execute_query_references_with_stats,
    reconstruct_point,
};
use crate::storage::FSEIndex;
use crate::tests::support::sort_points;

fn reconstruct_references(index: &FSEIndex, references: &[QueryResultReference]) -> Vec<Vector> {
    references
        .iter()
        .map(|reference| {
            let node = &index.nodes[reference.node_id];

            reconstruct_point(node, reference.row_index)
        })
        .collect()
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
