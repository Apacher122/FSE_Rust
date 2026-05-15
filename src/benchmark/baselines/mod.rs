//! Baseline query execution implementations.
//!
//! This module contains exact range-query baselines used to compare FSE against
//! conventional execution strategies.

pub mod baseline;
pub mod kd_tree;
pub mod r_tree;
pub mod scan;

pub use baseline::{
    BaselineComparisonLabels, BaselineKind, BaselineQueryReport, BaselineQueryStats,
    BaselineRegistry, FlatScanBaseline, RangeQueryBaseline, execute_range_baseline,
};

pub use kd_tree::KdTreeBaseline;
pub use r_tree::RTreeBaseline;
pub use scan::{FlatScanReport, FlatScanStats, flat_scan, flat_scan_with_stats};
