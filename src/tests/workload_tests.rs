use crate::benchmark::clustered_workload_cases;

#[test]
fn clustered_workload_cases_include_expected_cases() {
    let cases = clustered_workload_cases();

    let names: Vec<&str> = cases.iter().map(|case| case.name.as_str()).collect();

    assert_eq!(
        names,
        vec![
            "middle_cluster_selective",
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
