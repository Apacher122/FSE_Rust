//! Benchmark CLI parse state.

use crate::benchmark::{
    BaselineKind, BenchmarkBaselineSet, BenchmarkCsvOutputConfig, BenchmarkDatasetKind,
    BenchmarkSuiteConfig,
};

use super::parsing::{
    parse_baseline_kind, parse_dataset_kind, parse_fse_execution_mode, parse_positive_usize,
    parse_usize,
};
use super::types::{BenchmarkCliConfig, BenchmarkTerminalOutputMode};
use super::usage::benchmark_usage;

const SMALL_DATASET_DEFAULT_TARGET_LEAF_SIZE: usize = 8;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BenchmarkCliParseState {
    suite_config: BenchmarkSuiteConfig,
    baseline_selection: BaselineSelectionState,
    csv_output: BenchmarkCsvOutputConfig,
    terminal_output_mode: BenchmarkTerminalOutputMode,
    target_leaf_size_was_set: bool,
    max_leaf_size_was_set: bool,
}

impl Default for BenchmarkCliParseState {
    fn default() -> Self {
        Self {
            suite_config: BenchmarkSuiteConfig::default(),
            baseline_selection: BaselineSelectionState::default(),
            csv_output: BenchmarkCsvOutputConfig::default(),
            terminal_output_mode: BenchmarkTerminalOutputMode::default(),
            target_leaf_size_was_set: false,
            max_leaf_size_was_set: false,
        }
    }
}

impl BenchmarkCliParseState {
    pub(super) fn select_baseline(&mut self, value: &str) -> Result<(), String> {
        let baseline_kind = parse_baseline_kind(value)?;

        self.baseline_selection.select_single(baseline_kind)?;
        self.suite_config.baseline_kind = baseline_kind;

        Ok(())
    }

    pub(super) fn select_all_baselines(&mut self) -> Result<(), String> {
        self.baseline_selection.select_all_exact()
    }

    pub(super) fn set_dataset_kind(&mut self, value: &str) -> Result<(), String> {
        self.suite_config.dataset_kind = parse_dataset_kind(value)?;

        Ok(())
    }

    pub(super) fn set_timing_iterations(&mut self, value: &str) -> Result<(), String> {
        self.suite_config.timing_iterations = parse_positive_usize("--iterations", value)?;

        Ok(())
    }

    pub(super) fn set_target_leaf_size(&mut self, value: &str) -> Result<(), String> {
        self.suite_config.target_leaf_size = parse_positive_usize("--target-leaf-size", value)?;
        self.target_leaf_size_was_set = true;

        Ok(())
    }

    pub(super) fn set_max_leaf_size(&mut self, value: &str) -> Result<(), String> {
        let max_leaf_size = parse_positive_usize("--max-leaf-size", value)?;

        self.suite_config.max_leaf_size = max_leaf_size;
        self.max_leaf_size_was_set = true;

        if !self.target_leaf_size_was_set {
            // keep old behavior unless the caller explicitly splits the knobs
            self.suite_config.target_leaf_size = max_leaf_size;
        }

        Ok(())
    }

    pub(super) fn set_max_depth(&mut self, value: &str) -> Result<(), String> {
        self.suite_config.max_depth = parse_usize("--max-depth", value)?;

        Ok(())
    }

    pub(super) fn set_fse_execution_mode(&mut self, value: &str) -> Result<(), String> {
        self.suite_config.fse_execution_mode = parse_fse_execution_mode(value)?;

        Ok(())
    }

    pub(super) fn set_fse_parallel_min_retained_leaves(
        &mut self,
        value: &str,
    ) -> Result<(), String> {
        self.suite_config.fse_parallel_min_retained_leaves =
            parse_usize("--fse-parallel-min-leaves", value)?;

        Ok(())
    }

    pub(super) fn set_csv_summary_path(&mut self, value: String) {
        self.csv_output.set_summary_path(value);
    }

    pub(super) fn set_csv_workloads_path(&mut self, value: String) {
        self.csv_output.set_workloads_path(value);
    }

    pub(super) fn set_csv_low_selectivity_gap_path(&mut self, value: String) {
        self.csv_output.set_low_selectivity_gap_path(value);
    }

    pub(super) fn enable_debug_report(&mut self) {
        self.terminal_output_mode = BenchmarkTerminalOutputMode::DebugReport;
    }

    pub(super) fn finish(mut self) -> Result<BenchmarkCliConfig, String> {
        self.apply_dataset_default_leaf_policy();

        self.suite_config.validate_leaf_size_policy()?;

        let baseline_set = self
            .baseline_selection
            .into_baseline_set(self.suite_config.baseline_kind);

        // build this once at the edge so the parser state does not leak out
        let baseline_kinds = baseline_set.selected_kinds();

        Ok(BenchmarkCliConfig {
            suite_config: self.suite_config,
            baseline_set,
            baseline_kinds,
            csv_output: self.csv_output,
            terminal_output_mode: self.terminal_output_mode,
        })
    }

    fn apply_dataset_default_leaf_policy(&mut self) {
        if self.target_leaf_size_was_set || self.max_leaf_size_was_set {
            return;
        }

        if matches!(
            self.suite_config.dataset_kind,
            BenchmarkDatasetKind::SmallClustered2D
        ) {
            // gap-aware construction made 8/8 the better tiny benchmark default again
            self.suite_config.target_leaf_size = SMALL_DATASET_DEFAULT_TARGET_LEAF_SIZE;
        }
    }
}
