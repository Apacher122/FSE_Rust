//! Benchmark suite configuration.

use crate::benchmark::baselines::BaselineKind;
use crate::benchmark::reports::RepeatedTimingConfig;
use crate::benchmark::workloads::{
    QueryWorkloadCase, clustered_points_2d, clustered_workload_cases, large_clustered_points_2d,
    large_clustered_workload_cases,
};
use crate::build::BuildConfig;
use crate::math::Vector;
use crate::query::execution::DEFAULT_PARALLEL_MIN_RETAINED_LEAVES;
use crate::query::{QueryExecutionMode, QueryExecutionOptions};

/// Default target and maximum leaf cardinality used by benchmark runs.
pub const DEFAULT_BENCHMARK_LEAF_SIZE: usize = 8;

/// Default maximum build depth for the small clustered benchmark dataset.
pub const SMALL_DATASET_DEFAULT_MAX_DEPTH: usize = 8;

/// Default maximum build depth for the large clustered benchmark dataset.
///
/// The large dataset contains enough records that an eight-level binary tree
/// cannot satisfy the default hard leaf cardinality limit.
pub const LARGE_DATASET_DEFAULT_MAX_DEPTH: usize = 16;

/// Dataset selection for benchmark suite execution.
///
/// # Runtime Role
///
/// `BenchmarkDatasetKind` lets demos and benchmark runners switch between
/// small readable datasets and larger timing-oriented datasets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BenchmarkDatasetKind {
    /// Small deterministic clustered dataset.
    SmallClustered2D,

    /// Larger deterministic clustered dataset.
    LargeClustered2D,
}

/// Configuration for benchmark suite execution.
///
/// # Runtime Role
///
/// `BenchmarkSuiteConfig` keeps dataset, build, workload, baseline, timing, and
/// FSE execution choices in one place so benchmark runs can be reproduced and
/// adjusted consistently.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchmarkSuiteConfig {
    /// Dataset used for the benchmark run.
    pub dataset_kind: BenchmarkDatasetKind,

    /// Baseline used for comparison.
    pub baseline_kind: BaselineKind,

    /// Target number of records stored in each FSE leaf.
    ///
    /// # Runtime Role
    ///
    /// This is the soft refinement target used by the builder. Optional splits
    /// below the hard maximum still need to pass the configured split policy.
    pub target_leaf_size: usize,

    /// Hard maximum number of records allowed in each FSE leaf.
    pub max_leaf_size: usize,

    /// Maximum recursive build depth.
    pub max_depth: usize,

    /// Number of timing iterations used for repeated timing.
    pub timing_iterations: usize,

    /// FSE query execution mode used during benchmark comparisons.
    pub fse_execution_mode: QueryExecutionMode,

    /// Minimum retained-leaf count required before parallel FSE mode uses Rayon.
    pub fse_parallel_min_retained_leaves: usize,
}

impl BenchmarkSuiteConfig {
    /// Creates a benchmark suite configuration.
    ///
    /// # Runtime Role
    ///
    /// The target leaf size defaults to the hard maximum leaf size. This
    /// preserves the old benchmark behavior unless a caller explicitly sets a
    /// lower target.
    ///
    /// # Panics
    ///
    /// Panics when `max_leaf_size` or `timing_iterations` is zero.
    pub fn new(
        dataset_kind: BenchmarkDatasetKind,
        baseline_kind: BaselineKind,
        max_leaf_size: usize,
        max_depth: usize,
        timing_iterations: usize,
    ) -> Self {
        assert!(max_leaf_size > 0, "max_leaf_size must be greater than zero");
        assert!(
            timing_iterations > 0,
            "timing_iterations must be greater than zero"
        );

        Self {
            dataset_kind,
            baseline_kind,
            target_leaf_size: max_leaf_size,
            max_leaf_size,
            max_depth,
            timing_iterations,
            fse_execution_mode: QueryExecutionMode::Serial,
            fse_parallel_min_retained_leaves: DEFAULT_PARALLEL_MIN_RETAINED_LEAVES,
        }
    }

    /// Returns a copy of this configuration using the requested target leaf size.
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

    /// Returns a copy of this configuration using the requested FSE execution mode.
    pub fn with_fse_execution_mode(mut self, fse_execution_mode: QueryExecutionMode) -> Self {
        self.fse_execution_mode = fse_execution_mode;
        self
    }

    /// Returns a copy of this configuration using the requested parallel threshold.
    ///
    /// # Runtime Role
    ///
    /// This controls the retained-leaf count required before parallel FSE mode
    /// uses Rayon. Serial mode preserves this value but does not use it.
    pub fn with_fse_parallel_min_retained_leaves(
        mut self,
        fse_parallel_min_retained_leaves: usize,
    ) -> Self {
        self.fse_parallel_min_retained_leaves = fse_parallel_min_retained_leaves;
        self
    }

    /// Validates the benchmark leaf-size policy.
    ///
    /// # Runtime Role
    ///
    /// CLI parsing uses this after all arguments are read so flag order does not
    /// matter. For example, `--target-leaf-size 16 --max-leaf-size 32` should be
    /// accepted even though the default max leaf size is lower.
    pub fn validate_leaf_size_policy(&self) -> Result<(), String> {
        if self.target_leaf_size == 0 {
            return Err("`--target-leaf-size` must be greater than zero".to_string());
        }

        if self.max_leaf_size == 0 {
            return Err("`--max-leaf-size` must be greater than zero".to_string());
        }

        if self.target_leaf_size > self.max_leaf_size {
            return Err(format!(
                "`--target-leaf-size` ({}) must not exceed `--max-leaf-size` ({})",
                self.target_leaf_size, self.max_leaf_size
            ));
        }

        Ok(())
    }

    /// Returns the build configuration for this benchmark run.
    pub fn build_config(&self) -> BuildConfig {
        BuildConfig::new(self.max_leaf_size, self.max_depth)
            .with_target_leaf_size(self.target_leaf_size)
    }

    /// Returns the repeated timing configuration for this benchmark run.
    pub fn timing_config(&self) -> RepeatedTimingConfig {
        RepeatedTimingConfig::new(self.timing_iterations)
    }

    /// Returns the query execution options for this benchmark run.
    pub fn query_execution_options(&self) -> QueryExecutionOptions {
        let options = match self.fse_execution_mode {
            QueryExecutionMode::Serial => QueryExecutionOptions::serial(),
            QueryExecutionMode::Parallel => QueryExecutionOptions::parallel(),
        };

        // threshold lives in benchmark config so cli can poke it
        options.with_parallel_min_retained_leaves(self.fse_parallel_min_retained_leaves)
    }

    /// Returns the dataset associated with this benchmark configuration.
    pub fn dataset(&self) -> Vec<Vector> {
        match self.dataset_kind {
            BenchmarkDatasetKind::SmallClustered2D => clustered_points_2d(),
            BenchmarkDatasetKind::LargeClustered2D => large_clustered_points_2d(),
        }
    }

    /// Returns the workload cases associated with this benchmark configuration.
    pub fn workloads(&self) -> Vec<QueryWorkloadCase> {
        match self.dataset_kind {
            BenchmarkDatasetKind::SmallClustered2D => clustered_workload_cases(),
            BenchmarkDatasetKind::LargeClustered2D => large_clustered_workload_cases(),
        }
    }
}

impl Default for BenchmarkSuiteConfig {
    fn default() -> Self {
        Self {
            dataset_kind: BenchmarkDatasetKind::LargeClustered2D,
            baseline_kind: BaselineKind::FlatScan,
            target_leaf_size: DEFAULT_BENCHMARK_LEAF_SIZE,
            max_leaf_size: DEFAULT_BENCHMARK_LEAF_SIZE,
            max_depth: LARGE_DATASET_DEFAULT_MAX_DEPTH,
            timing_iterations: 10,
            fse_execution_mode: QueryExecutionMode::Serial,
            fse_parallel_min_retained_leaves: DEFAULT_PARALLEL_MIN_RETAINED_LEAVES,
        }
    }
}
