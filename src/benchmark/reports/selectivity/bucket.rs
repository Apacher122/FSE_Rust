//! Selectivity bucket classification.

use crate::math::Scalar;

/// Stable selectivity bucket for workload-level benchmark summaries.
///
/// # Runtime Role
///
/// `SelectivityBucket` groups workloads by the fraction of baseline records that
/// FSE reconstructs. This makes benchmark behavior easier to interpret across
/// query shapes instead of reading each workload row manually.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectivityBucket {
    /// Workloads that reconstruct no records.
    Empty,

    /// Workloads that reconstruct at most one quarter of baseline records.
    Low,

    /// Workloads that reconstruct at most half of baseline records.
    Medium,

    /// Workloads that reconstruct less than the full baseline record count.
    High,

    /// Workloads that reconstruct the full baseline record count or more.
    Full,
}

impl SelectivityBucket {
    /// Returns the bucket for a candidate ratio.
    ///
    /// # Runtime Role
    ///
    /// This classifier is intentionally based on candidate ratio rather than
    /// match ratio because reconstruction volume is the direct cost controlled
    /// by FSE pruning.
    pub fn from_candidate_ratio(candidate_ratio: Scalar) -> Self {
        if candidate_ratio <= 0.0 {
            return Self::Empty;
        }

        if candidate_ratio <= 0.25 {
            return Self::Low;
        }

        if candidate_ratio <= 0.50 {
            return Self::Medium;
        }

        if candidate_ratio < 1.0 {
            return Self::High;
        }

        Self::Full
    }

    /// Returns the stable label used in benchmark output.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Full => "full",
        }
    }
}

impl Default for SelectivityBucket {
    fn default() -> Self {
        Self::Empty
    }
}
