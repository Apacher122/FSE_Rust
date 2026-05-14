//! Simple exact R-tree baseline.

use crate::benchmark::{BaselineKind, BaselineQueryReport, BaselineQueryStats, RangeQueryBaseline};
use crate::math::{BoundingBox, Vector};
use crate::query::QueryRegion;

/// Exact R-tree range-query baseline.
///
/// # Runtime Role
///
/// `RTreeBaseline` provides a deterministic bounding hierarchy baseline for
/// comparing FSE against a classic spatial indexing structure.
///
/// # Notes
///
/// This implementation is intentionally simple. It bulk-loads points into
/// packed leaves and recursively groups bounding boxes into internal nodes.
#[derive(Clone, Debug, PartialEq)]
pub struct RTreeBaseline {
    root: Option<RTreeNode>,
    dimensions: usize,
}

impl RTreeBaseline {
    const MAX_LEAF_SIZE: usize = 16;
    const MAX_CHILDREN: usize = 8;

    /// Builds an R-tree baseline from source points.
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
                "all R-tree points must have the same dimensionality"
            );
        }

        Self {
            root: build_r_tree(points.to_vec()),
            dimensions,
        }
    }

    /// Returns the number of dimensions represented by the R-tree.
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    /// Returns true when the tree contains no points.
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }
}

impl RangeQueryBaseline for RTreeBaseline {
    fn name(&self) -> &'static str {
        BaselineKind::RTree.name()
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
            "query dimensionality must match R-tree dimensionality"
        );

        let query_bounds = query.as_bounds();
        let mut results = Vec::new();
        let mut stats = BaselineQueryStats::default();

        if let Some(root) = &self.root {
            query_r_tree_node(root, query, &query_bounds, &mut results, &mut stats);
        }

        BaselineQueryReport {
            baseline_name: self.name().to_string(),
            results,
            stats,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum RTreeNode {
    Leaf {
        bounds: BoundingBox,
        points: Vec<Vector>,
    },
    Internal {
        bounds: BoundingBox,
        children: Vec<RTreeNode>,
    },
}

impl RTreeNode {
    fn bounds(&self) -> &BoundingBox {
        match self {
            RTreeNode::Leaf { bounds, .. } => bounds,
            RTreeNode::Internal { bounds, .. } => bounds,
        }
    }

    fn internal(children: Vec<RTreeNode>) -> Self {
        let bounds = bounds_for_nodes(&children);

        RTreeNode::Internal { bounds, children }
    }
}

fn build_r_tree(mut points: Vec<Vector>) -> Option<RTreeNode> {
    if points.is_empty() {
        return None;
    }

    points.sort_by(|left, right| {
        left.values[0]
            .partial_cmp(&right.values[0])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut nodes = Vec::new();

    for chunk in points.chunks(RTreeBaseline::MAX_LEAF_SIZE) {
        let leaf_points = chunk.to_vec();
        let bounds = BoundingBox::from_points(&leaf_points);

        nodes.push(RTreeNode::Leaf {
            bounds,
            points: leaf_points,
        });
    }

    while nodes.len() > 1 {
        nodes = build_parent_level(nodes);
    }

    nodes.pop()
}

fn build_parent_level(mut nodes: Vec<RTreeNode>) -> Vec<RTreeNode> {
    nodes.sort_by(|left, right| {
        left.bounds().min[0]
            .partial_cmp(&right.bounds().min[0])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut parents = Vec::new();
    let mut group = Vec::with_capacity(RTreeBaseline::MAX_CHILDREN);

    for node in nodes {
        group.push(node);

        if group.len() == RTreeBaseline::MAX_CHILDREN {
            parents.push(RTreeNode::internal(group));
            group = Vec::with_capacity(RTreeBaseline::MAX_CHILDREN);
        }
    }

    if !group.is_empty() {
        parents.push(RTreeNode::internal(group));
    }

    parents
}

fn bounds_for_nodes(nodes: &[RTreeNode]) -> BoundingBox {
    assert!(
        !nodes.is_empty(),
        "cannot build bounds for empty R-tree node set"
    );

    let dimensions = nodes[0].bounds().dimensions();
    let mut min = vec![f32::INFINITY; dimensions];
    let mut max = vec![f32::NEG_INFINITY; dimensions];

    for node in nodes {
        let bounds = node.bounds();

        for dimension in 0..dimensions {
            min[dimension] = min[dimension].min(bounds.min[dimension]);
            max[dimension] = max[dimension].max(bounds.max[dimension]);
        }
    }

    BoundingBox::new(min, max)
}

fn query_r_tree_node(
    node: &RTreeNode,
    query: &QueryRegion,
    query_bounds: &BoundingBox,
    results: &mut Vec<Vector>,
    stats: &mut BaselineQueryStats,
) {
    if !node.bounds().intersects(query_bounds) {
        return;
    }

    match node {
        RTreeNode::Leaf { points, .. } => {
            for point in points {
                // Only points inside retained leaf boxes require exact evaluation.
                stats.evaluated_records += 1;

                if query.contains_point(point) {
                    stats.matched_records += 1;
                    results.push(point.clone());
                }
            }
        }
        RTreeNode::Internal { children, .. } => {
            for child in children {
                query_r_tree_node(child, query, query_bounds, results, stats);
            }
        }
    }
}
