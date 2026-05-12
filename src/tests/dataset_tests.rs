use crate::benchmark::clustered_points_2d;

#[test]
fn clustered_points_2d_generates_expected_record_count() {
    let points = clustered_points_2d();
    assert_eq!(points.len(), 60);
}

#[test]
fn clustered_points_2d_generates_expected_cluster_boundaries() {
    let points = clustered_points_2d();

    assert_eq!(points.first().unwrap().values, vec![0.0, 0.0]);
    assert_eq!(points[20].values, vec![50.0, 50.0]);
    assert_eq!(points[40].values, vec![100.0, 100.0]);
    assert_eq!(points.last().unwrap().values, vec![119.0, 119.0]);
}
