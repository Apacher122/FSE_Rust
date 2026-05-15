use crate::benchmark::{compare_query_execution, pruning_efficiency_report};
use crate::query::QueryRegion;
use crate::tests::support::small_benchmark_fixture;

fn middle_cluster_query() -> QueryRegion {
    QueryRegion::new(vec![50.0, 50.0], vec![55.0, 55.0])
}

#[test]
fn pruning_efficiency_report_matches_candidate_complement() {
    let fixture = small_benchmark_fixture();
    let query = middle_cluster_query();

    let comparison = compare_query_execution(&fixture.index, &fixture.points, &query);
    let pruning = pruning_efficiency_report(&comparison);

    assert_eq!(
        pruning.record_pruning_efficiency,
        1.0 - comparison.candidate_ratio
    );
}

#[test]
fn pruning_efficiency_report_matches_retained_leaf_complement() {
    let fixture = small_benchmark_fixture();
    let query = middle_cluster_query();

    let comparison = compare_query_execution(&fixture.index, &fixture.points, &query);
    let pruning = pruning_efficiency_report(&comparison);

    assert_eq!(
        pruning.leaf_pruning_efficiency,
        1.0 - comparison.retained_leaf_ratio
    );
}

#[test]
fn pruning_efficiency_report_counts_baseline_and_reconstructed_records() {
    let fixture = small_benchmark_fixture();
    let query = middle_cluster_query();

    let comparison = compare_query_execution(&fixture.index, &fixture.points, &query);
    let pruning = pruning_efficiency_report(&comparison);

    assert_eq!(
        pruning.baseline_records,
        comparison.baseline_stats.evaluated_records
    );
    assert_eq!(
        pruning.reconstructed_records,
        comparison.fse_stats.reconstructed_records
    );
}

#[test]
fn pruning_efficiency_report_counts_leaf_retention() {
    let fixture = small_benchmark_fixture();
    let query = middle_cluster_query();

    let comparison = compare_query_execution(&fixture.index, &fixture.points, &query);
    let pruning = pruning_efficiency_report(&comparison);

    assert_eq!(pruning.total_leaves, comparison.fse_stats.total_leaves);
    assert_eq!(
        pruning.retained_leaves,
        comparison.fse_stats.retained_leaves
    );
}
