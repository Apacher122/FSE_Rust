//! Baseline registry and execution dispatch.

use super::baseline::{BaselineKind, BaselineQueryReport, RangeQueryBaseline};
use super::flat_scan_baseline::FlatScanBaseline;
use super::kd_tree::KdTreeBaseline;
use super::r_tree::RTreeBaseline;
use crate::math::Vector;
use crate::query::QueryRegion;

/// Registry for constructing benchmark baselines.
///
/// # Runtime Role
///
/// `BaselineRegistry` centralizes baseline selection so benchmark configuration
/// can choose an implementation without hardcoding construction logic throughout
/// the runner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BaselineRegistry;

impl BaselineRegistry {
    /// Creates a baseline registry.
    pub fn new() -> Self {
        Self
    }

    /// Returns the baseline implementation for a configured baseline kind.
    pub fn resolve(&self, kind: BaselineKind, points: &[Vector]) -> Box<dyn RangeQueryBaseline> {
        match kind {
            BaselineKind::FlatScan => Box::new(FlatScanBaseline::new(points)),
            BaselineKind::KdTree => Box::new(KdTreeBaseline::new(points)),
            BaselineKind::RTree => Box::new(RTreeBaseline::new(points)),
        }
    }
}

/// Executes a range-query baseline.
///
/// # Runtime Role
///
/// This helper gives tests and benchmark runners a small common entrypoint for
/// baseline execution.
pub fn execute_range_baseline(
    baseline: &dyn RangeQueryBaseline,
    query: &QueryRegion,
) -> BaselineQueryReport {
    baseline.execute(query)
}
