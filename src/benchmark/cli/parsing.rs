//! Benchmark CLI parsing functions.

use crate::benchmark::{BaselineKind, BenchmarkDatasetKind, BenchmarkSuiteConfig};
use crate::query::QueryExecutionMode;

use super::state::BenchmarkCliParseState;
use super::types::BenchmarkCliConfig;
use super::usage::benchmark_usage;

/// Parses benchmark configuration from command-line arguments.
///
/// # Runtime Role
///
/// This parser intentionally supports a small set of benchmark flags without
/// introducing a command-line dependency. It is meant for local benchmark runs
/// and simple demo execution.
///
/// # Supported Flags
///
/// - `--baseline flat_scan`
/// - `--baseline kd_tree`
/// - `--baseline r_tree`
/// - `--all-baselines`
/// - `--dataset small`
/// - `--dataset large`
/// - `--iterations N`
/// - `--target-leaf-size N`
/// - `--max-leaf-size N`
/// - `--max-depth N`
/// - `--fse-execution serial`
/// - `--fse-execution parallel`
/// - `--fse-parallel-min-leaves N`
/// - `--csv-summary PATH`
/// - `--csv PATH`
/// - `--csv-workloads PATH`
/// - `--csv-low-selectivity-gap PATH`
/// - `--debug-report`
pub fn parse_benchmark_cli_config<I, S>(args: I) -> Result<BenchmarkCliConfig, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = BenchmarkCliParseState::default();
    let mut args = args.into_iter().map(Into::into).peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--baseline" => {
                let value = next_value(&mut args, "--baseline")?;
                state.select_baseline(&value)?;
            }
            "--all-baselines" => {
                state.select_all_baselines()?;
            }
            "--dataset" => {
                let value = next_value(&mut args, "--dataset")?;
                state.set_dataset_kind(&value)?;
            }
            "--iterations" => {
                let value = next_value(&mut args, "--iterations")?;
                state.set_timing_iterations(&value)?;
            }
            "--target-leaf-size" | "--leaf-target-size" => {
                let value = next_value(&mut args, arg.as_str())?;
                state.set_target_leaf_size(&value)?;
            }
            "--max-leaf-size" => {
                let value = next_value(&mut args, "--max-leaf-size")?;
                state.set_max_leaf_size(&value)?;
            }
            "--max-depth" => {
                let value = next_value(&mut args, "--max-depth")?;
                state.set_max_depth(&value)?;
            }
            "--fse-execution" | "--fse-mode" => {
                let value = next_value(&mut args, arg.as_str())?;
                state.set_fse_execution_mode(&value)?;
            }
            "--fse-parallel-min-leaves"
            | "--fse-parallel-min-retained-leaves"
            | "--fse-parallel-threshold" => {
                let value = next_value(&mut args, arg.as_str())?;

                // zero is valid so benchmarks can force rayon
                state.set_fse_parallel_min_retained_leaves(&value)?;
            }
            "--csv-summary" | "--csv" => {
                let value = next_value(&mut args, arg.as_str())?;
                state.set_csv_summary_path(value);
            }
            "--csv-workloads" => {
                let value = next_value(&mut args, "--csv-workloads")?;
                state.set_csv_workloads_path(value);
            }
            "--csv-low-selectivity-gap" | "--csv-low-gap" | "--csv-tree-gap" => {
                let value = next_value(&mut args, arg.as_str())?;
                state.set_csv_low_selectivity_gap_path(value);
            }
            "--debug-report" => {
                state.enable_debug_report();
            }
            "--help" | "-h" => {
                return Err(benchmark_usage());
            }
            unknown => {
                return Err(format!(
                    "unknown benchmark argument `{}`\n\n{}",
                    unknown,
                    benchmark_usage()
                ));
            }
        }
    }

    state.finish()
}

/// Parses benchmark suite configuration from command-line arguments.
///
/// # Runtime Role
///
/// This preserves the original single-suite configuration parser for tests and
/// callers that do not need multi-baseline selection.
pub fn parse_benchmark_config<I, S>(args: I) -> Result<BenchmarkSuiteConfig, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    parse_benchmark_cli_config(args).map(|config| config.suite_config)
}

fn next_value<I>(args: &mut std::iter::Peekable<I>, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| format!("missing value for `{}`\n\n{}", flag, benchmark_usage()))
}

pub(super) fn parse_baseline_kind(value: &str) -> Result<BaselineKind, String> {
    match value {
        "flat_scan" | "flat-scan" | "scan" => Ok(BaselineKind::FlatScan),
        "kd_tree" | "kd-tree" | "kdtree" => Ok(BaselineKind::KdTree),
        "r_tree" | "r-tree" | "rtree" => Ok(BaselineKind::RTree),
        other => Err(format!(
            "unsupported baseline `{}`\n\n{}",
            other,
            benchmark_usage()
        )),
    }
}

pub(super) fn parse_dataset_kind(value: &str) -> Result<BenchmarkDatasetKind, String> {
    match value {
        "small" | "small_clustered" | "small-clustered" => {
            Ok(BenchmarkDatasetKind::SmallClustered2D)
        }
        "large" | "large_clustered" | "large-clustered" => {
            Ok(BenchmarkDatasetKind::LargeClustered2D)
        }
        other => Err(format!(
            "unsupported dataset `{}`\n\n{}",
            other,
            benchmark_usage()
        )),
    }
}

pub(super) fn parse_fse_execution_mode(value: &str) -> Result<QueryExecutionMode, String> {
    match value {
        "serial" => Ok(QueryExecutionMode::Serial),
        "parallel" | "rayon" => Ok(QueryExecutionMode::Parallel),
        other => Err(format!(
            "unsupported FSE execution mode `{}`\n\n{}",
            other,
            benchmark_usage()
        )),
    }
}

pub(super) fn parse_positive_usize(flag: &str, value: &str) -> Result<usize, String> {
    let parsed = parse_usize(flag, value)?;

    if parsed == 0 {
        return Err(format!("`{}` must be greater than zero", flag));
    }

    Ok(parsed)
}

pub(super) fn parse_usize(flag: &str, value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("`{}` expects a numeric value, got `{}`", flag, value))
}
