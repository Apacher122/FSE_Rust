//! Simple exact KD-tree baseline.

use crate::benchmark::{BaselineKind, BaselineQueryReport, BaselineQueryStats, RangeQueryBaseline};
use crate::math::Vector;
use crate::query::QueryRegion;

/// Exact KD-tree range-query baseline.
///
/// # Runtime Role
///
/// `KdTreeBaseline` provides a deterministic spatial baseline for comparing FSE
/// against a classic recursive partitioning structure.
///
/// # Notes
///
/// This is intentionally simple. It is an exact range-query baseline, not an
/// approximate nearest-neighbor implementation.
#[derive(Clone, Debug, PartialEq)]
pub struct KdTreeBaseline {
    root: Option<Box<KdNode>>,
    dimensions: usize,
}

impl KdTreeBaseline {
    /// Builds a KD-tree baseline from source points.
    ///
    /// # Panics
    ///
    /// Panics when point dimensionality is inconsistent.
    pub fn new(points: &[Vector]) -> Self {
        if points.is_empty() {
            return Self {
                root: None,
                dimensions: 0,
            };
        }

        let dimensions = points[0].dimensions();

        for point in points {
            assert_eq!(
                point.dimensions(),
                dimensions,
                "all KD-tree points must have the same dimensionality"
            );
        }

        Self {
            root: build_kd_node(points.to_vec(), 0, dimensions),
            dimensions,
        }
    }

    /// Returns the number of dimensions represented by the KD-tree.
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    /// Returns true when the tree contains no points.
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }
}

impl RangeQueryBaseline for KdTreeBaseline {
    fn name(&self) -> &'static str {
        BaselineKind::KdTree.name()
    }

    fn execute(&self, query: &QueryRegion) -> BaselineQueryReport {
        if self.is_empty() {
            return BaselineQueryReport {
                baseline_name: self.name().to_string(),
                results: Vec::new(),
                stats: BaselineQueryStats::default(),
            };
        }

        assert_eq!(
            self.dimensions,
            query.dimensions(),
            "query dimensionality must match KD-tree dimensionality"
        );

        let mut results = Vec::new();
        let mut stats = BaselineQueryStats::default();

        if let Some(root) = &self.root {
            query_kd_node(root, query, &mut results, &mut stats);
        }

        BaselineQueryReport {
            baseline_name: self.name().to_string(),
            results,
            stats,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct KdNode {
    point: Vector,
    split_axis: usize,
    left: Option<Box<KdNode>>,
    right: Option<Box<KdNode>>,
}

fn build_kd_node(mut points: Vec<Vector>, depth: usize, dimensions: usize) -> Option<Box<KdNode>> {
    if points.is_empty() {
        return None;
    }

    let split_axis = depth % dimensions;

    points.sort_by(|left, right| {
        left.values[split_axis]
            .partial_cmp(&right.values[split_axis])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let median = points.len() / 2;
    let right_points = points.split_off(median + 1);
    let point = points.pop().expect("median point should exist");
    let left_points = points;

    Some(Box::new(KdNode {
        point,
        split_axis,
        left: build_kd_node(left_points, depth + 1, dimensions),
        right: build_kd_node(right_points, depth + 1, dimensions),
    }))
}

fn query_kd_node(
    node: &KdNode,
    query: &QueryRegion,
    results: &mut Vec<Vector>,
    stats: &mut BaselineQueryStats,
) {
    stats.evaluated_records += 1;

    if query.contains_point(&node.point) {
        stats.matched_records += 1;
        results.push(node.point.clone());
    }

    let axis = node.split_axis;
    let split_value = node.point.values[axis];

    // Axis checks decide which subtrees can still contain matching points.
    if query.min[axis] <= split_value {
        if let Some(left) = &node.left {
            query_kd_node(left, query, results, stats);
        }
    }

    if query.max[axis] >= split_value {
        if let Some(right) = &node.right {
            query_kd_node(right, query, results, stats);
        }
    }
}
