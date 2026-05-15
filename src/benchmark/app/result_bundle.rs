//! Completed benchmark application result bundle.

use super::context::BenchmarkApplicationContext;
use crate::benchmark::reports::{
    BenchmarkCsvMetadata, BenchmarkRunOverview, MultiBaselineAggregateSummary,
    summarize_multi_baseline_aggregates,
};
use crate::benchmark::runner::MultiBaselineBenchmarkSuiteReport;

/// Completed benchmark run data used by application rendering and output.
///
/// # Runtime Role
///
/// `BenchmarkApplicationResultBundle` groups the benchmark overview, full
/// per-baseline report, aggregate summary, and CSV metadata produced by a single
/// application run.
#[derive(Clone, Debug)]
pub struct BenchmarkApplicationResultBundle {
    /// Terminal and CSV overview metadata.
    pub overview: BenchmarkRunOverview,

    /// Full multi-baseline benchmark suite report.
    pub report: MultiBaselineBenchmarkSuiteReport,

    /// Aggregate summary derived from the full benchmark report.
    pub aggregate_summary: MultiBaselineAggregateSummary,

    /// Metadata used when writing CSV output.
    pub metadata: BenchmarkCsvMetadata,
}

impl BenchmarkApplicationResultBundle {
    /// Builds a result bundle by running the benchmark suite for the context.
    ///
    /// # Runtime Role
    ///
    /// This constructor owns the transition from prepared benchmark context to
    /// completed benchmark results.
    pub fn from_context(context: &BenchmarkApplicationContext) -> Self {
        let overview = context.overview();
        let report = context.run_suite();
        let aggregate_summary = summarize_multi_baseline_aggregates(&report);
        let metadata = BenchmarkCsvMetadata::from_overview(&overview);

        Self {
            overview,
            report,
            aggregate_summary,
            metadata,
        }
    }

    /// Returns the number of baseline reports produced by this run.
    pub fn baseline_report_count(&self) -> usize {
        self.report.baseline_reports.len()
    }
}
