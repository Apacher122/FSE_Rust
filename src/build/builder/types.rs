//! Builder result and internal split types.

use crate::build::metrics::SplitQualityMetrics;
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
    pub validation: crate::build::IndexValidationReport,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct AcceptedStructuralSplit {
    pub(super) left_points: Vec<Vector>,
    pub(super) right_points: Vec<Vector>,
    pub(super) metrics: SplitQualityMetrics,
    pub(super) was_forced: bool,
}
