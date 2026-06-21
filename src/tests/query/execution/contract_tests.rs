use crate::benchmark::flat_scan;
use crate::build::{BuildConfig, FSEBuilder};
use crate::math::Vector;
use crate::query::{
    QueryRegion, count_query_matches_with_stats, execute_query, execute_query_into,
    execute_query_references_with_stats, execute_query_with_stats, query_has_match_with_stats,
    reconstruct_query_result_references, visit_query_references, visit_query_row_views,
};
use crate::tests::support::sort_points;

#[test]
fn query_output_contracts_match_flat_scan_for_representative_workloads() {
    for case in representative_contract_cases() {
        assert_query_output_contracts_match_flat_scan(&case);
    }
}

struct QueryContractCase {
    name: &'static str,
    points: Vec<Vector>,
    query: QueryRegion,
    build_config: BuildConfig,
}

fn representative_contract_cases() -> Vec<QueryContractCase> {
    vec![
        QueryContractCase {
            name: "1d partial boundary range",
            points: vec![
                Vector::new(vec![-5.0]),
                Vector::new(vec![-1.0]),
                Vector::new(vec![0.0]),
                Vector::new(vec![1.0]),
                Vector::new(vec![2.0]),
                Vector::new(vec![8.0]),
            ],
            query: QueryRegion::new(vec![0.0], vec![2.0]),
            build_config: BuildConfig::new(2, 8),
        },
        QueryContractCase {
            name: "2d full coverage",
            points: vec![
                Vector::new(vec![-2.0, -1.0]),
                Vector::new(vec![0.0, 0.0]),
                Vector::new(vec![1.0, 3.0]),
                Vector::new(vec![4.0, 8.0]),
                Vector::new(vec![9.0, 9.0]),
            ],
            query: QueryRegion::new(vec![-10.0, -10.0], vec![10.0, 10.0]),
            build_config: BuildConfig::new(2, 8),
        },
        QueryContractCase {
            name: "2d disjoint range",
            points: vec![
                Vector::new(vec![0.0, 0.0]),
                Vector::new(vec![1.0, 1.0]),
                Vector::new(vec![2.0, 2.0]),
                Vector::new(vec![3.0, 3.0]),
            ],
            query: QueryRegion::new(vec![10.0, 10.0], vec![12.0, 12.0]),
            build_config: BuildConfig::new(2, 8),
        },
        QueryContractCase {
            name: "3d partial generic path",
            points: vec![
                Vector::new(vec![0.0, 0.0, 0.0]),
                Vector::new(vec![1.0, 2.0, 3.0]),
                Vector::new(vec![2.0, 4.0, 6.0]),
                Vector::new(vec![3.0, 6.0, 9.0]),
                Vector::new(vec![4.0, 8.0, 12.0]),
                Vector::new(vec![5.0, 10.0, 15.0]),
            ],
            query: QueryRegion::new(vec![1.0, 2.0, 3.0], vec![3.0, 6.0, 9.0]),
            build_config: BuildConfig::new(2, 8),
        },
    ]
}

fn assert_query_output_contracts_match_flat_scan(case: &QueryContractCase) {
    let index = FSEBuilder::new(case.build_config.clone()).build(&case.points);
    let mut expected_results = flat_scan(&case.points, &case.query);

    sort_points(&mut expected_results);

    assert_owned_results_match(case.name, &index, &case.query, &expected_results);
    assert_reusable_results_match(case.name, &index, &case.query, &expected_results);
    assert_reference_results_match(case.name, &index, &case.query, &expected_results);
    assert_reference_visitor_results_match(case.name, &index, &case.query, &expected_results);
    assert_row_view_visitor_results_match(case.name, &index, &case.query, &expected_results);
    assert_count_and_existence_match(case.name, &index, &case.query, &expected_results);
}

fn assert_owned_results_match(
    case_name: &str,
    index: &crate::storage::FSEIndex,
    query: &QueryRegion,
    expected_results: &[Vector],
) {
    let mut fresh_results = execute_query(index, query);
    sort_points(&mut fresh_results);

    assert_eq!(
        fresh_results, expected_results,
        "fresh owned query results differed from flat scan for case `{case_name}`"
    );

    let report = execute_query_with_stats(index, query);
    let mut reported_results = report.results;
    sort_points(&mut reported_results);

    assert_eq!(
        reported_results, expected_results,
        "owned query report results differed from flat scan for case `{case_name}`"
    );
    assert_eq!(
        report.stats.matched_records,
        expected_results.len(),
        "owned query matched count differed from flat scan for case `{case_name}`"
    );
}

fn assert_reusable_results_match(
    case_name: &str,
    index: &crate::storage::FSEIndex,
    query: &QueryRegion,
    expected_results: &[Vector],
) {
    let mut reusable_results = Vec::new();
    let stats = execute_query_into(index, query, &mut reusable_results);

    sort_points(&mut reusable_results);

    assert_eq!(
        reusable_results, expected_results,
        "reusable owned query results differed from flat scan for case `{case_name}`"
    );
    assert_eq!(
        stats.matched_records,
        expected_results.len(),
        "reusable owned query matched count differed from flat scan for case `{case_name}`"
    );
}

fn assert_reference_results_match(
    case_name: &str,
    index: &crate::storage::FSEIndex,
    query: &QueryRegion,
    expected_results: &[Vector],
) {
    let report = execute_query_references_with_stats(index, query);
    let mut reconstructed_results = reconstruct_query_result_references(index, &report.matches);

    sort_points(&mut reconstructed_results);

    assert_eq!(
        reconstructed_results, expected_results,
        "reference query results differed from flat scan for case `{case_name}`"
    );
    assert_eq!(
        report.stats.matched_records,
        expected_results.len(),
        "reference query matched count differed from flat scan for case `{case_name}`"
    );
}

fn assert_reference_visitor_results_match(
    case_name: &str,
    index: &crate::storage::FSEIndex,
    query: &QueryRegion,
    expected_results: &[Vector],
) {
    let mut references = Vec::new();
    let stats = visit_query_references(index, query, |reference| {
        references.push(reference);
    });

    let mut reconstructed_results = reconstruct_query_result_references(index, &references);
    sort_points(&mut reconstructed_results);

    assert_eq!(
        reconstructed_results, expected_results,
        "reference visitor results differed from flat scan for case `{case_name}`"
    );
    assert_eq!(
        stats.matched_records,
        expected_results.len(),
        "reference visitor matched count differed from flat scan for case `{case_name}`"
    );
}

fn assert_row_view_visitor_results_match(
    case_name: &str,
    index: &crate::storage::FSEIndex,
    query: &QueryRegion,
    expected_results: &[Vector],
) {
    let mut viewed_results = Vec::new();
    let stats = visit_query_row_views(index, query, |view| {
        viewed_results.push(view.to_vector());
    });

    sort_points(&mut viewed_results);

    assert_eq!(
        viewed_results, expected_results,
        "row-view visitor results differed from flat scan for case `{case_name}`"
    );
    assert_eq!(
        stats.matched_records,
        expected_results.len(),
        "row-view visitor matched count differed from flat scan for case `{case_name}`"
    );
}

fn assert_count_and_existence_match(
    case_name: &str,
    index: &crate::storage::FSEIndex,
    query: &QueryRegion,
    expected_results: &[Vector],
) {
    let count_report = count_query_matches_with_stats(index, query);
    let existence_report = query_has_match_with_stats(index, query);

    assert_eq!(
        count_report.matched_records,
        expected_results.len(),
        "count-only query differed from flat scan for case `{case_name}`"
    );
    assert_eq!(
        count_report.stats.matched_records,
        expected_results.len(),
        "count-only query stats differed from flat scan for case `{case_name}`"
    );
    assert_eq!(
        existence_report.has_match,
        !expected_results.is_empty(),
        "existence query differed from flat scan for case `{case_name}`"
    );
}
