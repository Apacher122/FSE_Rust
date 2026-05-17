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

    /// Maximum number of records stored in each FSE leaf.
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
            max_leaf_size,
            max_depth,
            timing_iterations,
            fse_execution_mode: QueryExecutionMode::Serial,
            fse_parallel_min_retained_leaves: DEFAULT_PARALLEL_MIN_RETAINED_LEAVES,
        }
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

    /// Returns the build configuration for this benchmark run.
    pub fn build_config(&self) -> BuildConfig {
        BuildConfig::new(self.max_leaf_size, self.max_depth)
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
            max_leaf_size: 8,
            max_depth: 8,
            timing_iterations: 10,
            fse_execution_mode: QueryExecutionMode::Serial,
            fse_parallel_min_retained_leaves: DEFAULT_PARALLEL_MIN_RETAINED_LEAVES,
        }
    }
}
