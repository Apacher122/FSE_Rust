use crate::benchmark::baselines::scan::flat_scan;
use crate::benchmark::baselines::{BaselineKind, BaselineRegistry, RangeQueryBaseline};
use crate::benchmark::reports::RepeatedTimingConfig;
use crate::benchmark::sort_points_lexicographically;
use crate::benchmark::workloads::{
    QueryWorkloadCase, clustered_points_2d, clustered_workload_cases, large_clustered_points_2d,
    large_clustered_workload_cases,
};
use crate::build::{BuildConfig, FSEBuilder};
use crate::math::Vector;
use crate::query::QueryRegion;
use crate::storage::FSEIndex;

/// Shared benchmark fixture used by benchmark-oriented tests.
pub struct BenchmarkTestFixture {
    pub points: Vec<Vector>,
    pub index: FSEIndex,
    pub workloads: Vec<QueryWorkloadCase>,
    pub timing_config: RepeatedTimingConfig,
    pub registry: BaselineRegistry,
}

/// Builds the standard small benchmark fixture used by unit tests.
pub fn small_benchmark_fixture() -> BenchmarkTestFixture {
    let points = clustered_points_2d();
    let index = build_test_index(&points);
    let workloads = clustered_workload_cases();
    let timing_config = RepeatedTimingConfig::new(3);
    let registry = BaselineRegistry::new();

    BenchmarkTestFixture {
        points,
        index,
        workloads,
        timing_config,
        registry,
    }
}

/// Builds the standard large benchmark fixture used by query workload tests.
pub fn large_benchmark_fixture() -> BenchmarkTestFixture {
    let points = large_clustered_points_2d();
    let builder = FSEBuilder::new(BuildConfig::new(8, 16).with_target_leaf_size(8));
    let index = builder.build(&points);
    let workloads = large_clustered_workload_cases();
    let timing_config = RepeatedTimingConfig::new(3);
    let registry = BaselineRegistry::new();

    BenchmarkTestFixture {
        points,
        index,
        workloads,
        timing_config,
        registry,
    }
}

/// Builds an FSE index using the standard test build configuration.
pub fn build_test_index(points: &[Vector]) -> FSEIndex {
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));

    builder.build(points)
}

/// Sorts points lexicographically for order-independent result comparison.
pub fn sort_points(points: &mut [Vector]) {
    sort_points_lexicographically(points);
}

/// Verifies that a baseline returns the same exact result set as flat scan.
pub fn assert_baseline_matches_flat_scan<B>(baseline: B, points: &[Vector], query: &QueryRegion)
where
    B: RangeQueryBaseline,
{
    let baseline_report = baseline.execute(query);
    let scan_results = flat_scan(points, query);

    let mut baseline_results = baseline_report.results;
    let mut expected_results = scan_results;

    sort_points(&mut baseline_results);
    sort_points(&mut expected_results);

    assert_eq!(baseline_results, expected_results);
    assert_eq!(
        baseline_report.stats.matched_records,
        expected_results.len()
    );
    assert!(baseline_report.stats.evaluated_records <= points.len());
}

/// Verifies that all selected baselines return identical results for each workload.
///
/// # Runtime Role
///
/// This helper tests baseline equivalence independently from FSE. It is useful
/// for catching correctness drift between exact baseline implementations before
/// those baselines are used as comparison references.
pub fn assert_baselines_match_for_workloads(
    baseline_kinds: &[BaselineKind],
    points: &[Vector],
    workloads: &[QueryWorkloadCase],
) {
    assert!(
        !baseline_kinds.is_empty(),
        "at least one baseline kind is required"
    );

    let registry = BaselineRegistry::new();

    for workload in workloads {
        let mut expected: Option<(BaselineKind, Vec<Vector>, usize)> = None;

        for baseline_kind in baseline_kinds {
            let baseline = registry.resolve(*baseline_kind, points);
            let report = baseline.execute(&workload.query);
            let mut results = report.results;

            // Traversal order is not the thing under test here.
            sort_points(&mut results);

            assert_eq!(
                report.stats.matched_records,
                results.len(),
                "baseline `{}` reported a matched count that does not match its result length",
                baseline_kind.name()
            );

            if let Some((expected_kind, expected_results, expected_matched_records)) = &expected {
                assert_eq!(
                    &results,
                    expected_results,
                    "baseline `{}` returned different results for workload `{}` than baseline `{}`",
                    baseline_kind.name(),
                    workload.name,
                    expected_kind.name()
                );

                assert_eq!(
                    report.stats.matched_records,
                    *expected_matched_records,
                    "baseline `{}` reported a different matched count for workload `{}` than baseline `{}`",
                    baseline_kind.name(),
                    workload.name,
                    expected_kind.name()
                );
            } else {
                // The first baseline becomes the reference row for this workload.
                expected = Some((*baseline_kind, results, report.stats.matched_records));
            }
        }
    }
}
