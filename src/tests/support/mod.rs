use crate::benchmark::baselines::scan::flat_scan;
use crate::benchmark::baselines::{BaselineRegistry, RangeQueryBaseline};
use crate::benchmark::reports::RepeatedTimingConfig;
use crate::benchmark::workloads::{
    QueryWorkloadCase, clustered_points_2d, clustered_workload_cases,
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

/// Builds an FSE index using the standard test build configuration.
pub fn build_test_index(points: &[Vector]) -> FSEIndex {
    let builder = FSEBuilder::new(BuildConfig::new(8, 8));

    builder.build(points)
}

/// Sorts points lexicographically for order-independent result comparison.
pub fn sort_points(points: &mut [Vector]) {
    points.sort_by(|left, right| {
        for (left_value, right_value) in left.values.iter().zip(&right.values) {
            match left_value.partial_cmp(right_value) {
                Some(std::cmp::Ordering::Equal) => continue,
                Some(ordering) => return ordering,
                None => return std::cmp::Ordering::Equal,
            }
        }

        left.values.len().cmp(&right.values.len())
    });
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
