//! Benchmark application setup context.

use crate::benchmark::baselines::{BaselineKind, BaselineRegistry, BenchmarkBaselineSet};
use crate::benchmark::cli::BenchmarkCliConfig;
use crate::benchmark::config::BenchmarkSuiteConfig;
use crate::benchmark::reports::BenchmarkRunOverview;
use crate::benchmark::reports::RepeatedTimingConfig;
use crate::benchmark::runner::{
    MultiBaselineBenchmarkSuiteReport, run_multi_baseline_benchmark_suite,
};
use crate::benchmark::workloads::QueryWorkloadCase;
use crate::build::{FSEBuilder, IndexValidationReport};
use crate::math::Vector;
use crate::storage::FSEIndex;

/// Built benchmark application context.
///
/// # Runtime Role
///
/// `BenchmarkApplicationContext` owns the configured dataset, workloads,
/// constructed FSE index, validation report, timing configuration, and selected
/// baselines needed for one benchmark application run.
#[derive(Clone, Debug)]
pub struct BenchmarkApplicationContext {
    /// Benchmark suite configuration.
    pub suite_config: BenchmarkSuiteConfig,

    /// Named baseline selection requested by the caller.
    pub baseline_set: BenchmarkBaselineSet,

    /// Concrete baseline kinds selected for execution.
    pub baseline_kinds: Vec<BaselineKind>,

    /// CSV output paths selected for this benchmark run.
    pub csv_output: crate::benchmark::reports::BenchmarkCsvOutputConfig,

    /// Dataset records used by the benchmark run.
    pub points: Vec<Vector>,

    /// Query workloads used by the benchmark run.
    pub workloads: Vec<QueryWorkloadCase>,

    /// Repeated timing configuration used by benchmark comparisons.
    pub timing_config: RepeatedTimingConfig,

    /// Constructed FSE index.
    pub index: FSEIndex,

    /// Validation report for the constructed FSE index.
    pub validation: IndexValidationReport,

    /// Registry used to construct benchmark baselines.
    pub registry: BaselineRegistry,
}

impl BenchmarkApplicationContext {
    /// Builds a benchmark application context from parsed CLI configuration.
    ///
    /// # Runtime Role
    ///
    /// This constructor performs all setup required before benchmark execution:
    /// dataset selection, workload selection, timing configuration, index
    /// construction, validation, and baseline registry initialization.
    pub fn from_cli_config(cli_config: BenchmarkCliConfig) -> Self {
        let BenchmarkCliConfig {
            suite_config,
            baseline_set,
            baseline_kinds,
            csv_output,
        } = cli_config;

        let points = suite_config.dataset();
        let workloads = suite_config.workloads();
        let timing_config = suite_config.timing_config();

        // build happens once here so the rest of the app uses ready state
        let builder = FSEBuilder::new(suite_config.build_config());
        let validated = builder.build_validated(&points);

        Self {
            suite_config,
            baseline_set,
            baseline_kinds,
            csv_output,
            points,
            workloads,
            timing_config,
            index: validated.index,
            validation: validated.validation,
            registry: BaselineRegistry::new(),
        }
    }

    /// Returns whether this context represents a multi-baseline run.
    pub fn has_multiple_baselines(&self) -> bool {
        self.baseline_set.is_multi_baseline()
    }

    /// Builds the terminal and CSV metadata overview for this benchmark run.
    pub fn overview(&self) -> BenchmarkRunOverview {
        BenchmarkRunOverview {
            dataset_records: self.points.len(),
            index_nodes: self.index.node_count(),
            workloads: self.workloads.len(),
            baselines: self.baseline_set.selected_name_list(),
            timing_iterations: self.timing_config.iterations,
            max_leaf_size: self.suite_config.max_leaf_size,
            max_depth: self.suite_config.max_depth,
            validation: self.validation.clone(),
        }
    }

    /// Runs the configured multi-baseline benchmark suite.
    pub fn run_suite(&self) -> MultiBaselineBenchmarkSuiteReport {
        run_multi_baseline_benchmark_suite(
            &self.index,
            &self.points,
            &self.workloads,
            &self.timing_config,
            &self.registry,
            &self.baseline_kinds,
        )
    }
}
