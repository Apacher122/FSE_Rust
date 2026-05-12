//! Lightweight timing utilities for benchmark execution.

use std::time::{Duration, Instant};

/// Wall-clock timing report for one comparison run.
///
/// # Runtime Role
///
/// `TimingReport` records elapsed time for the baseline scan path and the FSE
/// query path. These measurements are intended for demos and early regression
/// checks, not statistically rigorous benchmarking.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TimingReport {
    /// Elapsed time spent executing the flat scan baseline.
    pub flat_scan_elapsed: Duration,

    /// Elapsed time spent executing the FSE query path.
    pub fse_elapsed: Duration,
}

/// Runs a function and returns its output with elapsed wall-clock time.
///
/// # Runtime Role
///
/// This helper keeps timing instrumentation local to the benchmark module.
pub fn measure_elapsed<T>(operation: impl FnOnce() -> T) -> (T, Duration) {
    let started_at = Instant::now();
    let output = operation();
    let elapsed = started_at.elapsed();

    (output, elapsed)
}
