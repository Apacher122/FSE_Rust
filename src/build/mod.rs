//! Index construction components.
//!
//! This module contains the initial builder pipeline for constructing an FSE
//! hierarchy from embedded coordinate vectors.

pub mod builder;
pub mod metrics;
pub mod splitter;
pub mod validation;
pub mod validation_diagnostics;
pub mod variance;

pub use builder::{BuildConfig, FSEBuilder, ValidatedFSEIndex};
pub use metrics::{
    IndexStructureMetrics, SiblingOverlapMetrics, SplitQualityMetrics, bounding_extent_sum,
    index_density, index_structure_metrics, partition_density, sibling_overlap_extent_sum,
    sibling_overlap_metrics, split_quality_metrics, split_quality_metrics_for_axis,
    split_quality_metrics_from_bounds,
};
pub use validation::{
    IndexValidationReport, validate_hierarchy_topology, validate_index, validate_leaf_cardinality,
    validate_leaf_record_bounds, validate_parent_child_bounds,
};
pub use validation_diagnostics::{
    HierarchyTopologyDiagnostics, IndexValidationDiagnostics, InvalidChildReference,
    LeafCardinalityViolation, ParentChildBoundsViolation, index_validation_diagnostics,
};
