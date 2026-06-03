//! Builder result and internal split types.

use std::error::Error;
use std::fmt;

use crate::build::metrics::SplitQualityMetrics;
use crate::build::validation::IndexValidationReport;
use crate::build::validation_diagnostics::IndexValidationDiagnostics;
use crate::math::{CentroidError, Vector};
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

/// Error returned when checked builder input validation fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuildInputError {
    /// No coordinate vectors were provided.
    EmptyPointSet,
    /// Point validation failed before recursive construction.
    Points(CentroidError),
}

impl fmt::Display for BuildInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPointSet => formatter.write_str("cannot build an index from empty points"),
            Self::Points(error) => error.fmt(formatter),
        }
    }
}

impl Error for BuildInputError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::EmptyPointSet => None,
            Self::Points(error) => Some(error),
        }
    }
}

impl From<CentroidError> for BuildInputError {
    fn from(error: CentroidError) -> Self {
        match error {
            CentroidError::EmptyPointSet => Self::EmptyPointSet,
            other => Self::Points(other),
        }
    }
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

/// Error returned by strict checked index construction.
#[derive(Clone, Debug, PartialEq)]
pub enum BuildCheckedError {
    /// Builder input validation failed.
    Input(BuildInputError),
    /// Constructed output failed index validation.
    Validation(BuildValidationError),
}

impl fmt::Display for BuildCheckedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input(error) => error.fmt(formatter),
            Self::Validation(error) => error.fmt(formatter),
        }
    }
}

impl Error for BuildCheckedError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Input(error) => Some(error),
            Self::Validation(error) => Some(error),
        }
    }
}

impl From<BuildInputError> for BuildCheckedError {
    fn from(error: BuildInputError) -> Self {
        Self::Input(error)
    }
}

impl From<BuildValidationError> for BuildCheckedError {
    fn from(error: BuildValidationError) -> Self {
        Self::Validation(error)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct AcceptedStructuralSplit {
    pub(super) left_points: Vec<Vector>,
    pub(super) right_points: Vec<Vector>,
    pub(super) metrics: SplitQualityMetrics,
    pub(super) was_forced: bool,
}
