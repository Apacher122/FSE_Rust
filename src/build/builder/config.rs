//! Builder configuration.

use std::error::Error;
use std::fmt;

/// Error returned when checked build configuration fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuildConfigError {
    /// The hard maximum leaf size was zero.
    MaxLeafSizeZero,
    /// The target leaf size was zero.
    TargetLeafSizeZero,
    /// The target leaf size exceeded the hard maximum leaf size.
    TargetLeafSizeExceedsMax {
        /// Requested target leaf size.
        target_leaf_size: usize,
        /// Hard maximum leaf size.
        max_leaf_size: usize,
    },
}

impl fmt::Display for BuildConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MaxLeafSizeZero => formatter.write_str("max_leaf_size must be greater than zero"),
            Self::TargetLeafSizeZero => {
                formatter.write_str("target_leaf_size must be greater than zero")
            }
            Self::TargetLeafSizeExceedsMax { .. } => {
                formatter.write_str("target_leaf_size must not exceed max_leaf_size")
            }
        }
    }
}

impl Error for BuildConfigError {}

/// Configuration for recursive FSE index construction.
#[derive(Clone, Debug, PartialEq)]
pub struct BuildConfig {
    /// Target number of records stored in a leaf partition.
    ///
    /// # Runtime Role
    ///
    /// The builder starts considering subdivision when a partition exceeds this
    /// target. If the partition is still below the hard maximum, geometric split
    /// quality can decide whether subdivision is worthwhile.
    pub target_leaf_size: usize,

    /// Hard maximum number of records allowed in a leaf partition.
    ///
    /// # Runtime Role
    ///
    /// This value is the validation limit. Partitions above this size are forced
    /// to split when depth still allows it, even if the candidate split does not
    /// improve bounding volume.
    pub max_leaf_size: usize,

    /// Maximum recursive depth allowed during construction.
    pub max_depth: usize,

    /// Whether optional recursive subdivision requires geometric improvement.
    ///
    /// # Runtime Role
    ///
    /// When enabled, the builder rejects optional splits whose child bounds do
    /// not improve the parent geometry. Normal partitions require volume
    /// improvement. Degenerate zero-volume partitions can use extent improvement
    /// as the fallback geometric signal.
    pub require_positive_split_volume_reduction: bool,
}

impl BuildConfig {
    /// Creates a new build configuration.
    ///
    /// # Runtime Role
    ///
    /// The default target leaf size is equal to the hard maximum leaf size. This
    /// preserves the previous constructor behavior while allowing callers to
    /// lower the target size later for optional density-aware refinement.
    ///
    /// # Panics
    ///
    /// Panics when `max_leaf_size` is zero.
    pub fn new(max_leaf_size: usize, max_depth: usize) -> Self {
        Self::try_new(max_leaf_size, max_depth).unwrap_or_else(|error| panic!("{error}"))
    }

    /// Creates a new build configuration and returns an error when invalid.
    ///
    /// # Runtime Role
    ///
    /// This constructor is intended for caller-facing input validation. It
    /// enforces the same invariants as [`BuildConfig::new`] without panicking.
    pub fn try_new(max_leaf_size: usize, max_depth: usize) -> Result<Self, BuildConfigError> {
        if max_leaf_size == 0 {
            return Err(BuildConfigError::MaxLeafSizeZero);
        }

        Ok(Self {
            target_leaf_size: max_leaf_size,
            max_leaf_size,
            max_depth,
            require_positive_split_volume_reduction: true,
        })
    }

    /// Returns a copy of this configuration with a different target leaf size.
    ///
    /// # Runtime Role
    ///
    /// A target below the hard maximum lets the builder attempt optional
    /// density-aware refinement while still preserving the hard validation limit.
    ///
    /// # Panics
    ///
    /// Panics when `target_leaf_size` is zero or greater than `max_leaf_size`.
    pub fn with_target_leaf_size(self, target_leaf_size: usize) -> Self {
        self.try_with_target_leaf_size(target_leaf_size)
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// Returns a checked copy of this configuration with a different target leaf size.
    ///
    /// # Runtime Role
    ///
    /// This method is intended for caller-facing input validation. It enforces
    /// the same invariants as [`BuildConfig::with_target_leaf_size`] without
    /// panicking.
    pub fn try_with_target_leaf_size(
        mut self,
        target_leaf_size: usize,
    ) -> Result<Self, BuildConfigError> {
        if target_leaf_size == 0 {
            return Err(BuildConfigError::TargetLeafSizeZero);
        }

        if target_leaf_size > self.max_leaf_size {
            return Err(BuildConfigError::TargetLeafSizeExceedsMax {
                target_leaf_size,
                max_leaf_size: self.max_leaf_size,
            });
        }

        self.target_leaf_size = target_leaf_size;
        Ok(self)
    }

    /// Returns a copy of this configuration with split-volume gating changed.
    ///
    /// # Runtime Role
    ///
    /// This is primarily useful for tests and controlled experiments. Production
    /// benchmarking should normally keep the geometry gate enabled so optional
    /// subdivision does not grow the hierarchy without a geometric reason.
    pub fn with_positive_split_volume_reduction_required(mut self, required: bool) -> Self {
        self.require_positive_split_volume_reduction = required;
        self
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            target_leaf_size: 64,
            max_leaf_size: 64,
            max_depth: 32,
            require_positive_split_volume_reduction: true,
        }
    }
}
