//! Baseline query execution implementations.
//!
//! This module contains exact range-query baselines used to compare FSE against
//! conventional execution strategies.

pub mod baseline;
mod flat_scan_baseline;
pub mod kd_tree;
pub mod r_tree;
mod registry;
pub mod scan;

pub use baseline::{
    BaselineComparisonLabels, BaselineKind, BaselineQueryReport, BaselineQueryStats,
    BenchmarkBaselineSet, EXACT_RANGE_BASELINE_KINDS, RangeQueryBaseline, baseline_kind_name_list,
    baseline_kind_names, exact_range_baseline_kinds, exact_range_baseline_vec,
    has_multiple_baselines,
};

pub use flat_scan_baseline::FlatScanBaseline;
pub use kd_tree::KdTreeBaseline;
pub use r_tree::RTreeBaseline;
pub use registry::{BaselineRegistry, execute_range_baseline};
pub use scan::{FlatScanReport, FlatScanStats, flat_scan, flat_scan_with_stats};
