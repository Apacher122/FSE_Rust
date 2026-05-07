use crate::benchmark::clustered_points_2d;

#[test]
fn clustered_points_2d_generates_expected_record_count() {
    let points = clustered_points_2d();
    assert_eq!(points.len(), 60);
}
