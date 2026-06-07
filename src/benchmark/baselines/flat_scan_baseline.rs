//! Flat scan baseline implementation.

use super::baseline::{BaselineKind, BaselineQueryReport, RangeQueryBaseline};
use super::footprint::BaselineFootprintMetrics;
use super::scan::flat_scan_with_stats;
use crate::math::Vector;
use crate::query::QueryRegion;

/// Flat scan baseline implementation.
///
/// # Runtime Role
///
/// `FlatScanBaseline` represents the conventional full-record evaluation path.
#[derive(Clone, Debug, PartialEq)]
pub struct FlatScanBaseline {
    points: Vec<Vector>,
}

impl FlatScanBaseline {
    /// Creates a flat scan baseline from source points.
    pub fn new(points: &[Vector]) -> Self {
        Self {
            points: points.to_vec(),
        }
    }

    /// Returns the number of records represented by the baseline.
    pub fn record_count(&self) -> usize {
        self.points.len()
    }

    /// Returns the dimensionality of the represented coordinate space.
    pub fn dimensions(&self) -> usize {
        self.points.first().map_or(0, Vector::dimensions)
    }
}

impl RangeQueryBaseline for FlatScanBaseline {
    fn name(&self) -> &'static str {
        BaselineKind::FlatScan.name()
    }

    fn footprint_metrics(&self) -> BaselineFootprintMetrics {
        BaselineFootprintMetrics::flat_scan(self.record_count(), self.dimensions())
    }

    fn execute(&self, query: &QueryRegion) -> BaselineQueryReport {
        // keep the first baseline intentionally boring every record gets checked
        let report = flat_scan_with_stats(&self.points, query);

        BaselineQueryReport {
            baseline_name: self.name().to_string(),
            results: report.results,
            stats: report.stats.into(),
        }
    }
}
