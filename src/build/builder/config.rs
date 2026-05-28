//! Builder configuration.

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
        assert!(max_leaf_size > 0, "max_leaf_size must be greater than zero");

        Self {
            target_leaf_size: max_leaf_size,
            max_leaf_size,
            max_depth,
            require_positive_split_volume_reduction: true,
        }
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
    pub fn with_target_leaf_size(mut self, target_leaf_size: usize) -> Self {
        assert!(
            target_leaf_size > 0,
            "target_leaf_size must be greater than zero"
        );
        assert!(
            target_leaf_size <= self.max_leaf_size,
            "target_leaf_size must not exceed max_leaf_size"
        );

        self.target_leaf_size = target_leaf_size;
        self
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
