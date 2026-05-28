//! Split input validation helpers.

use crate::math::Vector;

pub(super) fn validate_points_for_split(points: &[Vector]) -> usize {
    assert!(
        points.len() >= 2,
        "median split requires at least two points"
    );

    let dimensions = points[0].dimensions();
    assert!(dimensions > 0, "points must have at least one dimension");

    for point in points {
        assert_eq!(
            point.dimensions(),
            dimensions,
            "all points must have the same dimensionality"
        );
    }

    dimensions
}
