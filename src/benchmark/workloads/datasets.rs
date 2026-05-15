//! Deterministic benchmark datasets.

use crate::math::{Scalar, Vector};

/// Configuration for deterministic two-dimensional clustered dataset generation.
///
/// # Runtime Role
///
/// `ClusteredDatasetConfig` controls the number, spacing, and size of generated
/// clusters used by demos and benchmark runs.
#[derive(Clone, Debug, PartialEq)]
pub struct ClusteredDatasetConfig {
    /// Number of clusters to generate.
    pub cluster_count: usize,

    /// Number of points generated per cluster.
    pub points_per_cluster: usize,

    /// Distance between each cluster origin.
    pub cluster_spacing: Scalar,

    /// Per-point offset within each cluster.
    pub point_step: Scalar,
}

impl ClusteredDatasetConfig {
    /// Creates a deterministic clustered dataset configuration.
    ///
    /// # Panics
    ///
    /// Panics when `cluster_count` or `points_per_cluster` is zero.
    pub fn new(
        cluster_count: usize,
        points_per_cluster: usize,
        cluster_spacing: Scalar,
        point_step: Scalar,
    ) -> Self {
        assert!(cluster_count > 0, "cluster_count must be greater than zero");
        assert!(
            points_per_cluster > 0,
            "points_per_cluster must be greater than zero"
        );

        Self {
            cluster_count,
            points_per_cluster,
            cluster_spacing,
            point_step,
        }
    }
}

/// Generates a deterministic two-dimensional clustered dataset.
///
/// # Runtime Role
///
/// This generator creates separated diagonal clusters so geometric pruning can
/// be observed in a repeatable way.
pub fn generate_clustered_points_2d(config: &ClusteredDatasetConfig) -> Vec<Vector> {
    let mut points = Vec::with_capacity(config.cluster_count * config.points_per_cluster);

    for cluster_index in 0..config.cluster_count {
        let base = cluster_index as Scalar * config.cluster_spacing;

        // Keep clusters separated so range queries can exercise pruning behavior.
        append_cluster(
            &mut points,
            base,
            base,
            config.points_per_cluster,
            config.point_step,
        );
    }

    points
}

/// Generates a small deterministic two-dimensional clustered dataset.
///
/// # Runtime Role
///
/// This dataset is intended for examples, smoke tests, and readable demo output.
/// It deliberately creates separated clusters so geometric pruning can be
/// observed clearly.
pub fn clustered_points_2d() -> Vec<Vector> {
    generate_clustered_points_2d(&ClusteredDatasetConfig::new(3, 20, 50.0, 1.0))
}

/// Generates a larger deterministic two-dimensional clustered dataset.
///
/// # Runtime Role
///
/// This dataset is intended for timing demos and early benchmark runs where the
/// small demo dataset is too small to produce meaningful elapsed-time output.
pub fn large_clustered_points_2d() -> Vec<Vector> {
    generate_clustered_points_2d(&ClusteredDatasetConfig::new(10, 1_000, 1_000.0, 0.5))
}

fn append_cluster(
    points: &mut Vec<Vector>,
    base_x: Scalar,
    base_y: Scalar,
    count: usize,
    point_step: Scalar,
) {
    for offset in 0..count {
        let offset = offset as Scalar * point_step;

        points.push(Vector::new(vec![base_x + offset, base_y + offset]));
    }
}
