use crate::benchmark::{
    RangeWorkloadConfig, clustered_workload_cases, generate_range_workload_cases,
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
