//! Command-line parsing for benchmark configuration.

use crate::benchmark::{BaselineKind, BenchmarkDatasetKind, BenchmarkSuiteConfig};

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
/// - `--dataset small`
/// - `--dataset large`
/// - `--iterations N`
/// - `--max-leaf-size N`
/// - `--max-depth N`
pub fn parse_benchmark_config<I, S>(args: I) -> Result<BenchmarkSuiteConfig, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut config = BenchmarkSuiteConfig::default();
    let mut args = args.into_iter().map(Into::into).peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--baseline" => {
                let value = next_value(&mut args, "--baseline")?;
                config.baseline_kind = parse_baseline_kind(&value)?;
            }
            "--dataset" => {
                let value = next_value(&mut args, "--dataset")?;
                config.dataset_kind = parse_dataset_kind(&value)?;
            }
            "--iterations" => {
                let value = next_value(&mut args, "--iterations")?;
                config.timing_iterations = parse_positive_usize("--iterations", &value)?;
            }
            "--max-leaf-size" => {
                let value = next_value(&mut args, "--max-leaf-size")?;
                config.max_leaf_size = parse_positive_usize("--max-leaf-size", &value)?;
            }
            "--max-depth" => {
                let value = next_value(&mut args, "--max-depth")?;
                config.max_depth = parse_usize("--max-depth", &value)?;
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

    Ok(config)
}

/// Returns benchmark CLI usage text.
pub fn benchmark_usage() -> String {
    [
        "Usage:",
        "  cargo run --release -- [options]",
        "",
        "Options:",
        "  --baseline <flat_scan|kd_tree>",
        "  --dataset <small|large>",
        "  --iterations <N>",
        "  --max-leaf-size <N>",
        "  --max-depth <N>",
    ]
    .join("\n")
}

fn next_value<I>(args: &mut std::iter::Peekable<I>, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| format!("missing value for `{}`\n\n{}", flag, benchmark_usage()))
}

fn parse_baseline_kind(value: &str) -> Result<BaselineKind, String> {
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

fn parse_dataset_kind(value: &str) -> Result<BenchmarkDatasetKind, String> {
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

fn parse_positive_usize(flag: &str, value: &str) -> Result<usize, String> {
    let parsed = parse_usize(flag, value)?;

    if parsed == 0 {
        return Err(format!("`{}` must be greater than zero", flag));
    }

    Ok(parsed)
}

fn parse_usize(flag: &str, value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("`{}` expects a numeric value, got `{}`", flag, value))
}
