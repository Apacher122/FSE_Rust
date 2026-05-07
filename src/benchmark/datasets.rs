//! Deterministic benchmark datasets.

use crate::math::Vector;

/// Generates a deterministic two-dimensional clustered dataset.
///
/// # Runtime Role
///
/// This dataset is intended for examples, smoke tests, and early benchmark
/// comparisons. It deliberately creates separated clusters so geometric pruning
/// can be observed more clearly.
///
/// # Dataset Shape
///
/// The generated points form three clusters:
///
/// - low cluster near `[0, 0]`
/// - middle cluster near `[50, 50]`
/// - high cluster near `[100, 100]`
pub fn clustered_points_2d() -> Vec<Vector> {
    let mut points = Vec::new();
    append_cluster(&mut points, 0.0, 0.0, 20);
    append_cluster(&mut points, 50.0, 50.0, 20);
    append_cluster(&mut points, 100.0, 100.0, 20);
    points
}

fn append_cluster(points: &mut Vec<Vector>, base_x: f32, base_y: f32, count: usize) {
    for offset in 0..count {
        let offset = offset as f32;
        points.push(Vector::new(vec![base_x + offset, base_y + offset]));
    }
}
