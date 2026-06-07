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

pub use builder::{
    BuildCheckedError, BuildConfig, BuildConfigError, BuildInputError, BuildValidationError,
    FSEBuilder, RowMappedFSEIndex, ValidatedFSEIndex,
};
pub use metrics::{
    IndexFootprintByteEstimates, IndexFootprintComparisonMetrics, IndexFootprintMetrics,
    IndexStructureMetrics, SiblingOverlapMetrics, SplitQualityMetrics, bounding_extent_sum,
    footprint_byte_estimates, footprint_comparison_metrics, index_density,
    index_footprint_byte_estimates, index_footprint_comparison_metrics, index_footprint_metrics,
    index_structure_metrics, partition_density, sibling_overlap_extent_sum,
    sibling_overlap_metrics, split_quality_metrics, split_quality_metrics_for_axis,
    split_quality_metrics_from_bounds,
};
pub use validation::{
    IndexValidationReport, validate_hierarchy_topology, validate_index, validate_leaf_cardinality,
    validate_leaf_ownership_cardinality, validate_leaf_reconstruction_metadata,
    validate_leaf_record_bounds, validate_node_identifier_consistency,
    validate_parent_child_bounds, validate_partition_dimensional_metadata,
};
pub use validation_diagnostics::{
    HierarchyTopologyDiagnostics, IndexValidationDiagnostics, InvalidChildReference,
    LeafCardinalityViolation, LeafOwnershipCardinalityDiagnostics,
    LeafOwnershipCardinalityViolation, LeafOwnershipParentCountViolation,
    LeafReconstructionLeafCountMismatch, LeafReconstructionMetadataDiagnostics,
    LeafReconstructionShapeListLengthMismatch, LeafReconstructionShapeListMismatch,
    LeafReconstructionShapeLookupLengthMismatch, LeafReconstructionShapeLookupMismatch,
    LeafRecordBoundsViolation, NodeIdentifierMismatch, ParentChildBoundsViolation,
    PartitionDimensionalMetadataDiagnostics, PartitionDimensionalMetadataViolation,
    index_validation_diagnostics,
};
