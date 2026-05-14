use crate::benchmark::{
    RangeWorkloadConfig, clustered_workload_cases, generate_range_workload_cases,
    large_clustered_workload_cases,
};

#[test]
fn clustered_workload_cases_include_expected_cases() {
    let cases = clustered_workload_cases();

    let names: Vec<&str> = cases.iter().map(|case| case.name.as_str()).collect();

    assert_eq!(
        names,
        vec![
            "cluster_range_000",
            "cluster_range_001",
            "cluster_range_002",
            "empty_far_range",
            "full_dataset_range",
            "cluster_boundary_range",
        ]
    );
}

#[test]
fn clustered_workload_cases_have_two_dimensional_queries() {
    let cases = clustered_workload_cases();

    for case in cases {
        assert_eq!(case.query.dimensions(), 2);
    }
}

#[test]
fn large_clustered_workload_cases_include_expected_case_count() {
    let cases = large_clustered_workload_cases();

    assert_eq!(cases.len(), 13);
}

#[test]
fn large_clustered_workload_cases_include_expected_names() {
    let cases = large_clustered_workload_cases();

    let names: Vec<&str> = cases.iter().map(|case| case.name.as_str()).collect();

    assert_eq!(
        names,
        vec![
            "large_cluster_range_000",
            "large_cluster_range_001",
            "large_cluster_range_002",
            "large_cluster_range_003",
            "large_cluster_range_004",
            "large_cluster_range_005",
            "large_cluster_range_006",
            "large_cluster_range_007",
            "large_cluster_range_008",
            "large_cluster_range_009",
            "large_empty_far_range",
            "large_full_dataset_range",
            "large_cross_cluster_boundary",
        ]
    );
}

#[test]
fn large_clustered_workload_cases_have_two_dimensional_queries() {
    let cases = large_clustered_workload_cases();

    for case in cases {
        assert_eq!(case.query.dimensions(), 2);
    }
}

#[test]
fn large_clustered_workload_cases_target_expected_cluster_ranges() {
    let cases = large_clustered_workload_cases();

    assert_eq!(cases[0].query.min, vec![0.0, 0.0]);
    assert_eq!(cases[0].query.max, vec![25.0, 25.0]);

    assert_eq!(cases[1].query.min, vec![1000.0, 1000.0]);
    assert_eq!(cases[1].query.max, vec![1025.0, 1025.0]);

    assert_eq!(cases[9].query.min, vec![9000.0, 9000.0]);
    assert_eq!(cases[9].query.max, vec![9025.0, 9025.0]);
}

#[test]
fn range_workload_generator_creates_expected_number_of_cases() {
    let config =
        RangeWorkloadConfig::new("range", 4, vec![0.0, 0.0], vec![10.0, 10.0], vec![5.0, 5.0]);

    let cases = generate_range_workload_cases(&config);

    assert_eq!(cases.len(), 4);
}

#[test]
fn range_workload_generator_creates_stable_names() {
    let config =
        RangeWorkloadConfig::new("range", 3, vec![0.0, 0.0], vec![10.0, 10.0], vec![5.0, 5.0]);

    let cases = generate_range_workload_cases(&config);
    let names: Vec<&str> = cases.iter().map(|case| case.name.as_str()).collect();

    assert_eq!(names, vec!["range_000", "range_001", "range_002"]);
}

#[test]
fn range_workload_generator_creates_expected_query_bounds() {
    let config =
        RangeWorkloadConfig::new("range", 3, vec![0.0, 0.0], vec![10.0, 20.0], vec![5.0, 8.0]);

    let cases = generate_range_workload_cases(&config);

    assert_eq!(cases[0].query.min, vec![0.0, 0.0]);
    assert_eq!(cases[0].query.max, vec![5.0, 8.0]);

    assert_eq!(cases[1].query.min, vec![10.0, 20.0]);
    assert_eq!(cases[1].query.max, vec![15.0, 28.0]);

    assert_eq!(cases[2].query.min, vec![20.0, 40.0]);
    assert_eq!(cases[2].query.max, vec![25.0, 48.0]);
}

#[test]
fn range_workload_config_reports_dimensions() {
    let config = RangeWorkloadConfig::new(
        "range",
        2,
        vec![0.0, 0.0, 0.0],
        vec![1.0, 1.0, 1.0],
        vec![2.0, 2.0, 2.0],
    );

    assert_eq!(config.dimensions(), 3);
}
