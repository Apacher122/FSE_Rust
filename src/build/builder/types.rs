//! Builder result and internal split types.

use std::error::Error;
use std::fmt;

use crate::build::metrics::SplitQualityMetrics;
use crate::build::validation::IndexValidationReport;
use crate::build::validation_diagnostics::IndexValidationDiagnostics;
use crate::math::Vector;
use crate::storage::FSEIndex;

/// Builder output paired with validation results.
///
/// # Runtime Role
///
/// `ValidatedFSEIndex` is useful when construction should immediately report
/// whether the generated index satisfies core hierarchy invariants.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedFSEIndex {
    /// Constructed FSE index.
    pub index: FSEIndex,

    /// Validation report for the constructed index.
    pub validation: IndexValidationReport,
}

/// Error returned when checked construction produces an invalid index.
///
/// # Runtime Role
///
/// `BuildValidationError` preserves the constructed index, compact validation
/// report, and detailed diagnostics so callers can decide whether to inspect,
/// log, or discard the failed build.
#[derive(Clone, Debug, PartialEq)]
pub struct BuildValidationError {
    /// Constructed index that failed validation.
    pub index: FSEIndex,

    /// Compact validation report for the failed build.
    pub validation: IndexValidationReport,

    /// Detailed validation diagnostics for the failed build.
    pub diagnostics: IndexValidationDiagnostics,
}

impl BuildValidationError {
    /// Creates a checked-build validation error from a validated index.
    pub fn new(validated: ValidatedFSEIndex, diagnostics: IndexValidationDiagnostics) -> Self {
        Self {
            index: validated.index,
            validation: validated.validation,
            diagnostics,
        }
    }

    /// Returns the failed build as a report-only validated index.
    pub fn into_validated(self) -> ValidatedFSEIndex {
        ValidatedFSEIndex {
            index: self.index,
            validation: self.validation,
        }
    }

    fn failed_validation_checks(&self) -> Vec<&'static str> {
        let mut failed = Vec::new();

        if !self.validation.node_identifier_consistency_valid {
            failed.push("node identifier consistency");
        }

        if !self.validation.partition_dimensional_metadata_valid {
            failed.push("partition dimensional metadata");
        }

        if !self.validation.leaf_cardinality_valid {
            failed.push("leaf cardinality");
        }

        if !self.validation.leaf_reconstruction_metadata_valid {
            failed.push("leaf reconstruction metadata");
        }

        if !self.validation.leaf_record_bounds_valid {
            failed.push("leaf record bounds");
        }

        if !self.validation.leaf_ownership_cardinality_valid {
            failed.push("leaf ownership cardinality");
        }

        if !self.validation.hierarchy_topology_valid {
            failed.push("hierarchy topology");
        }

        if !self.validation.parent_child_bounds_valid {
            failed.push("parent-child bounds");
        }

        failed
    }
}

impl fmt::Display for BuildValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let failed_checks = self.failed_validation_checks();

        if failed_checks.is_empty() {
            formatter.write_str("constructed FSE index failed validation")
        } else {
            write!(
                formatter,
                "constructed FSE index failed validation: {}",
                failed_checks.join(", ")
            )
        }
    }
}

impl Error for BuildValidationError {}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct AcceptedStructuralSplit {
    pub(super) left_points: Vec<Vector>,
    pub(super) right_points: Vec<Vector>,
    pub(super) metrics: SplitQualityMetrics,
    pub(super) was_forced: bool,
}
