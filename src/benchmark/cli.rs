//! Command-line parsing for benchmark configuration.

use crate::benchmark::{
    BaselineKind, BenchmarkBaselineSet, BenchmarkDatasetKind, BenchmarkSuiteConfig,
};

/// Parsed benchmark CLI configuration.
///
/// # Runtime Role
///
/// `BenchmarkCliConfig` separates the benchmark suite configuration from the
/// selected baseline list. This allows the CLI to support both single-baseline
/// and multi-baseline benchmark runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchmarkCliConfig {
    /// Benchmark suite configuration.
    pub suite_config: BenchmarkSuiteConfig,

    /// Named baseline selection requested by the CLI.
    pub baseline_set: BenchmarkBaselineSet,

    /// Baselines selected for this benchmark run.
    pub baseline_kinds: Vec<BaselineKind>,

    /// Optional path for writing the aggregate summary CSV.
    pub csv_summary_path: Option<String>,

    /// Optional path for writing per-workload CSV rows.
    pub csv_workloads_path: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BaselineSelectionState {
    Default,
    Single(BaselineKind),
    AllExact,
}

impl BaselineSelectionState {
    fn select_single(&mut self, baseline_kind: BaselineKind) -> Result<(), String> {
        if matches!(self, BaselineSelectionState::AllExact) {
            return Err(format!(
                "`--baseline` cannot be combined with `--all-baselines`\n\n{}",
                benchmark_usage()
            ));
        }

        // last baseline wins same as before just keep the rule contained
        *self = BaselineSelectionState::Single(baseline_kind);

        Ok(())
    }

    fn select_all_exact(&mut self) -> Result<(), String> {
        if matches!(self, BaselineSelectionState::Single(_)) {
            return Err(format!(
                "`--all-baselines` cannot be combined with `--baseline`\n\n{}",
                benchmark_usage()
            ));
        }

        *self = BaselineSelectionState::AllExact;

        Ok(())
    }

    fn into_baseline_set(self, default_baseline: BaselineKind) -> BenchmarkBaselineSet {
        match self {
            BaselineSelectionState::Default => BenchmarkBaselineSet::Single(default_baseline),
            BaselineSelectionState::Single(baseline_kind) => {
                BenchmarkBaselineSet::Single(baseline_kind)
            }
            BaselineSelectionState::AllExact => BenchmarkBaselineSet::AllExact,
        }
    }
}

impl Default for BaselineSelectionState {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct BenchmarkCliParseState {
    suite_config: BenchmarkSuiteConfig,
    baseline_selection: BaselineSelectionState,
    csv_summary_path: Option<String>,
    csv_workloads_path: Option<String>,
}

impl BenchmarkCliParseState {
    fn select_baseline(&mut self, value: &str) -> Result<(), String> {
        let baseline_kind = parse_baseline_kind(value)?;

        self.baseline_selection.select_single(baseline_kind)?;
        self.suite_config.baseline_kind = baseline_kind;

        Ok(())
    }

    fn select_all_baselines(&mut self) -> Result<(), String> {
        self.baseline_selection.select_all_exact()
    }

    fn set_dataset_kind(&mut self, value: &str) -> Result<(), String> {
        self.suite_config.dataset_kind = parse_dataset_kind(value)?;

        Ok(())
    }

    fn set_timing_iterations(&mut self, value: &str) -> Result<(), String> {
        self.suite_config.timing_iterations = parse_positive_usize("--iterations", value)?;

        Ok(())
    }

    fn set_max_leaf_size(&mut self, value: &str) -> Result<(), String> {
        self.suite_config.max_leaf_size = parse_positive_usize("--max-leaf-size", value)?;

        Ok(())
    }

    fn set_max_depth(&mut self, value: &str) -> Result<(), String> {
        self.suite_config.max_depth = parse_usize("--max-depth", value)?;

        Ok(())
    }

    fn set_csv_summary_path(&mut self, value: String) {
        self.csv_summary_path = Some(value);
    }

    fn set_csv_workloads_path(&mut self, value: String) {
        self.csv_workloads_path = Some(value);
    }

    fn finish(self) -> BenchmarkCliConfig {
        let baseline_set = self
            .baseline_selection
            .into_baseline_set(self.suite_config.baseline_kind);

        // build this once at the edge so the parser state does not leak out
        let baseline_kinds = baseline_set.selected_kinds();

        BenchmarkCliConfig {
            suite_config: self.suite_config,
            baseline_set,
            baseline_kinds,
            csv_summary_path: self.csv_summary_path,
            csv_workloads_path: self.csv_workloads_path,
        }
    }
}

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
/// - `--max-leaf-size N`
/// - `--max-depth N`
/// - `--csv-summary PATH`
/// - `--csv PATH`
/// - `--csv-workloads PATH`
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
            "--max-leaf-size" => {
                let value = next_value(&mut args, "--max-leaf-size")?;
                state.set_max_leaf_size(&value)?;
            }
            "--max-depth" => {
                let value = next_value(&mut args, "--max-depth")?;
                state.set_max_depth(&value)?;
            }
            "--csv-summary" | "--csv" => {
                let value = next_value(&mut args, arg.as_str())?;
                state.set_csv_summary_path(value);
            }
            "--csv-workloads" => {
                let value = next_value(&mut args, "--csv-workloads")?;
                state.set_csv_workloads_path(value);
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

    Ok(state.finish())
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

/// Returns benchmark CLI usage text.
pub fn benchmark_usage() -> String {
    [
        "Usage:",
        "  cargo run --release -- [options]",
        "",
        "Options:",
        "  --baseline <flat_scan|kd_tree|r_tree>",
        "  --all-baselines",
        "  --dataset <small|large>",
        "  --iterations <N>",
        "  --max-leaf-size <N>",
        "  --max-depth <N>",
        "  --csv-summary <PATH>",
        "  --csv <PATH>",
        "  --csv-workloads <PATH>",
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
