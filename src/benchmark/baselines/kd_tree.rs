//! Simple exact KD-tree baseline.

use super::footprint::BaselineFootprintMetrics;
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

    /// Returns the number of records represented by the KD-tree.
    pub fn record_count(&self) -> usize {
        self.node_count()
    }

    /// Returns the number of nodes represented by the KD-tree.
    pub fn node_count(&self) -> usize {
        kd_node_counts(&self.root).node_count
    }

    /// Returns the number of leaf nodes represented by the KD-tree.
    pub fn leaf_count(&self) -> usize {
        kd_node_counts(&self.root).leaf_count
    }

    /// Returns logical footprint metrics for the KD-tree.
    pub fn footprint_metrics(&self) -> BaselineFootprintMetrics {
        let counts = kd_node_counts(&self.root);

        BaselineFootprintMetrics::kd_tree(
            counts.node_count,
            self.dimensions,
            counts.node_count,
            counts.leaf_count,
        )
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

    fn footprint_metrics(&self) -> BaselineFootprintMetrics {
        KdTreeBaseline::footprint_metrics(self)
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct KdNodeCounts {
    node_count: usize,
    leaf_count: usize,
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

fn kd_node_counts(root: &Option<Box<KdNode>>) -> KdNodeCounts {
    root.as_deref()
        .map_or_else(KdNodeCounts::default, count_kd_node)
}

fn count_kd_node(node: &KdNode) -> KdNodeCounts {
    let left_counts = kd_node_counts(&node.left);
    let right_counts = kd_node_counts(&node.right);
    let has_child = node.left.is_some() || node.right.is_some();

    KdNodeCounts {
        node_count: 1 + left_counts.node_count + right_counts.node_count,
        leaf_count: usize::from(!has_child) + left_counts.leaf_count + right_counts.leaf_count,
    }
}
