//! Benchmark CLI public types.

use crate::benchmark::{
    BaselineKind, BenchmarkBaselineSet, BenchmarkCsvOutputConfig, BenchmarkSuiteConfig,
};

/// Terminal output mode selected for a benchmark run.
///
/// # Runtime Role
///
/// `BenchmarkTerminalOutputMode` controls whether the benchmark application
/// prints a compact scoreboard or the full per-workload debug report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchmarkTerminalOutputMode {
    /// Print the compact benchmark scoreboard.
    Summary,

    /// Print the full per-workload debug report.
    DebugReport,
}

impl BenchmarkTerminalOutputMode {
    /// Returns whether detailed per-workload output should be rendered.
    pub fn is_debug_report(&self) -> bool {
        matches!(self, Self::DebugReport)
    }
}

impl Default for BenchmarkTerminalOutputMode {
    fn default() -> Self {
        Self::Summary
    }
}

/// Parsed benchmark CLI configuration.
///
/// # Runtime Role
///
/// `BenchmarkCliConfig` separates the benchmark suite configuration from the
/// selected baseline list. This allows the CLI to support both single-baseline
/// and multi-baseline benchmark runs while carrying optional artifact output
/// paths selected by the caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchmarkCliConfig {
    /// Benchmark suite configuration.
    pub suite_config: BenchmarkSuiteConfig,

    /// Named baseline selection requested by the CLI.
    pub baseline_set: BenchmarkBaselineSet,

    /// Baselines selected for this benchmark run.
    pub baseline_kinds: Vec<BaselineKind>,

    /// CSV output paths selected for this benchmark run.
    pub csv_output: BenchmarkCsvOutputConfig,

    /// Optional path for writing a typed query index archive artifact.
    pub typed_query_index_archive_path: Option<String>,

    /// Terminal output mode selected for this benchmark run.
    pub terminal_output_mode: BenchmarkTerminalOutputMode,
}
