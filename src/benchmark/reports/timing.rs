//! Lightweight timing utilities for benchmark execution.

use std::time::{Duration, Instant};

/// Wall-clock timing report for one comparison run.
///
/// # Runtime Role
///
/// `TimingReport` records elapsed time for the baseline query path and the FSE
/// query path. These measurements are intended for demos and early regression
/// checks, not statistically rigorous benchmarking.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TimingReport {
    /// Elapsed time spent executing the baseline query path.
    pub baseline_elapsed: Duration,

    /// Elapsed time spent executing the FSE query path.
    pub fse_elapsed: Duration,
}

/// Configuration for repeated timing measurements.
///
/// # Runtime Role
///
/// `RepeatedTimingConfig` controls how many times an operation should be timed
/// before reporting an average duration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepeatedTimingConfig {
    /// Number of measured iterations.
    pub iterations: usize,
}

impl RepeatedTimingConfig {
    /// Creates a repeated timing configuration.
    ///
    /// # Panics
    ///
    /// Panics when `iterations` is zero.
    pub fn new(iterations: usize) -> Self {
        assert!(
            iterations > 0,
            "timing iterations must be greater than zero"
        );

        Self { iterations }
    }
}

impl Default for RepeatedTimingConfig {
    fn default() -> Self {
        Self { iterations: 10 }
    }
}

/// Timing report collected over repeated runs.
///
/// # Runtime Role
///
/// This report stores the total and average elapsed time for repeated execution
/// of one operation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RepeatedTimingReport {
    /// Number of measured iterations.
    pub iterations: usize,

    /// Total elapsed time across all measured iterations.
    pub total_elapsed: Duration,

    /// Average elapsed time per measured iteration.
    pub average_elapsed: Duration,
}

/// Side-by-side repeated timing report for baseline and FSE execution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RepeatedComparisonTimingReport {
    /// Repeated timing report for baseline execution.
    pub baseline: RepeatedTimingReport,

    /// Repeated timing report for FSE execution.
    pub fse: RepeatedTimingReport,
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

/// Measures repeated execution time for an operation.
///
/// # Runtime Role
///
/// This helper is used when benchmark output should report averaged timing
/// instead of a single noisy elapsed duration.
pub fn measure_repeated(
    config: &RepeatedTimingConfig,
    mut operation: impl FnMut(),
) -> RepeatedTimingReport {
    let mut total_elapsed = Duration::ZERO;

    for _ in 0..config.iterations {
        total_elapsed += measure_iteration(&mut operation);
    }

    repeated_timing_report(config.iterations, total_elapsed)
}

/// Measures repeated baseline and FSE execution with alternating order.
///
/// # Runtime Role
///
/// `measure_repeated_comparison_interleaved` keeps repeated benchmark timing
/// fairer for very small operations by avoiding one large baseline timing block
/// followed by one large FSE timing block. Each measured iteration runs both
/// operations, but the order flips every other iteration:
///
/// ```text
/// iteration 0: baseline -> fse
/// iteration 1: fse -> baseline
/// iteration 2: baseline -> fse
/// iteration 3: fse -> baseline
/// ```
///
/// This does not change execution semantics or result reporting. It only changes
/// how repeated wall-clock timing samples are collected.
pub fn measure_repeated_comparison_interleaved(
    config: &RepeatedTimingConfig,
    mut baseline_operation: impl FnMut(),
    mut fse_operation: impl FnMut(),
) -> RepeatedComparisonTimingReport {
    let mut baseline_total_elapsed = Duration::ZERO;
    let mut fse_total_elapsed = Duration::ZERO;

    for iteration in 0..config.iterations {
        if iteration % 2 == 0 {
            baseline_total_elapsed += measure_iteration(&mut baseline_operation);
            fse_total_elapsed += measure_iteration(&mut fse_operation);
        } else {
            fse_total_elapsed += measure_iteration(&mut fse_operation);
            baseline_total_elapsed += measure_iteration(&mut baseline_operation);
        }
    }

    RepeatedComparisonTimingReport {
        baseline: repeated_timing_report(config.iterations, baseline_total_elapsed),
        fse: repeated_timing_report(config.iterations, fse_total_elapsed),
    }
}

/// Computes a ratio between two durations.
///
/// # Runtime Role
///
/// This is used to report timing ratios such as baseline elapsed time divided
/// by FSE elapsed time.
pub fn duration_ratio(numerator: Duration, denominator: Duration) -> f64 {
    if denominator == Duration::ZERO {
        if numerator == Duration::ZERO {
            return 0.0;
        }
        return f64::INFINITY;
    }

    numerator.as_secs_f64() / denominator.as_secs_f64()
}

fn measure_iteration(operation: &mut impl FnMut()) -> Duration {
    let started_at = Instant::now();
    operation();
    started_at.elapsed()
}

fn repeated_timing_report(iterations: usize, total_elapsed: Duration) -> RepeatedTimingReport {
    RepeatedTimingReport {
        iterations,
        total_elapsed,
        average_elapsed: duration_div(total_elapsed, iterations),
    }
}

fn duration_div(duration: Duration, divisor: usize) -> Duration {
    if divisor == 0 {
        return Duration::ZERO;
    }

    // duration division is kept explicit so the averaging logic is easy to audit
    Duration::from_secs_f64(duration.as_secs_f64() / divisor as f64)
}
