use crate::benchmark::{
    ClusteredDatasetConfig, clustered_points_2d, generate_clustered_points_2d,
    large_clustered_points_2d,
};

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

#[test]
fn large_clustered_points_2d_generates_expected_record_count() {
    let points = large_clustered_points_2d();

    assert_eq!(points.len(), 10_000);
}

#[test]
fn large_clustered_points_2d_generates_expected_cluster_boundaries() {
    let points = large_clustered_points_2d();

    assert_eq!(points.first().unwrap().values, vec![0.0, 0.0]);
    assert_eq!(points[1_000].values, vec![1000.0, 1000.0]);
    assert_eq!(points[9_000].values, vec![9000.0, 9000.0]);
    assert_eq!(points.last().unwrap().values, vec![9499.5, 9499.5]);
}

#[test]
fn generate_clustered_points_2d_uses_configured_shape() {
    let config = ClusteredDatasetConfig::new(2, 3, 10.0, 2.0);
    let points = generate_clustered_points_2d(&config);

    assert_eq!(points.len(), 6);

    assert_eq!(points[0].values, vec![0.0, 0.0]);
    assert_eq!(points[1].values, vec![2.0, 2.0]);
    assert_eq!(points[2].values, vec![4.0, 4.0]);

    assert_eq!(points[3].values, vec![10.0, 10.0]);
    assert_eq!(points[4].values, vec![12.0, 12.0]);
    assert_eq!(points[5].values, vec![14.0, 14.0]);
}
