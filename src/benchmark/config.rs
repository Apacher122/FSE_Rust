//! Benchmark suite configuration

use crate::benchmark::{
    QueryWorkloadCase, RepeatedTimingConfig, clustered_points_2d, clustered_workload_cases,
    large_clustered_points_2d, large_clustered_workload_cases,
};
use crate::build::BuildConfig;
use crate::math::Vector;

/// Dataset selection for benchmark suite execution.
///
/// # Runtime Role
///
/// `BenchmarkDatasetKind` lets demos and benchmark switch between
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
/// `BenchmarkSuiteConfig` keeps dataset, build, workload, and timing choices
/// in one place so benchmarks can be reproduced and adjusted consistently.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchmarkSuiteConfig {
    /// Dataset used for the benchmark run.
    pub dataset_kind: BenchmarkDatasetKind,

    /// Maximum number of records stored in each FSE leaf.
    pub max_leaf_size: usize,

    /// Maximum recursive build depth.
    pub max_depth: usize,

    /// Number of timing iterations used for repeated timing.
    pub timing_iterations: usize,
}

impl BenchmarkSuiteConfig {
    /// Creates a benchmark suite configuration.
    ///
    /// # Panics
    ///
    /// Panics when `max_leaf_size` or `timing_iterations` is zero.
    pub fn new(
        dataset_kind: BenchmarkDatasetKind,
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
            max_leaf_size,
            max_depth,
            timing_iterations,
        }
    }

    /// Returns the build configuration for this benchmark run.
    pub fn build_config(&self) -> BuildConfig {
        BuildConfig::new(self.max_leaf_size, self.max_depth)
    }

    /// Returns the repeated timing configurations for this benchmark run.
    pub fn timing_config(&self) -> RepeatedTimingConfig {
        RepeatedTimingConfig::new(self.timing_iterations)
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
        Self::new(BenchmarkDatasetKind::LargeClustered2D, 8, 8, 10)
    }
}
