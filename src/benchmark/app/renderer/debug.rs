//! Debug benchmark terminal rendering.

mod selectivity;
mod structure;
mod suite;

use super::BenchmarkApplicationRenderer;
use crate::benchmark::reports::render_benchmark_overview;

use super::super::context::BenchmarkApplicationContext;
use super::super::result_bundle::BenchmarkApplicationResultBundle;

impl BenchmarkApplicationRenderer {
    pub(super) fn render_debug_terminal_output(
        &self,
        context: &BenchmarkApplicationContext,
        result_bundle: &BenchmarkApplicationResultBundle,
    ) -> String {
        let mut output = String::new();

        output.push_str(&render_benchmark_overview(&result_bundle.overview));
        self.append_sibling_overlap_debug_output(&mut output, context);
        self.append_traversal_pressure_debug_output(&mut output, result_bundle);
        self.append_low_selectivity_tree_gap_debug_output(&mut output, result_bundle);
        self.append_weakest_low_selectivity_workload_debug_output(&mut output, result_bundle);
        self.append_boundary_workload_pressure_debug_output(&mut output, result_bundle);
        // keep this as debug-only until the boundary data says what to optimize
        self.append_target_workload_retained_leaf_debug_output(&mut output, context);
        self.append_target_workload_stage_timing_debug_output(&mut output, context);
        self.append_target_workload_reconstruction_timing_debug_output(&mut output, context);
        self.append_target_workload_retained_execution_phase_debug_output(&mut output, context);
        self.append_target_workload_retained_allocation_debug_output(&mut output, context);
        self.append_target_workload_resultless_timing_debug_output(&mut output, context);
        self.append_target_workload_materialization_mode_debug_output(&mut output, context);
        self.append_target_workload_existence_timing_debug_output(&mut output, context);
        self.append_target_workload_typed_indexed_comparison_debug_output(&mut output, context);
        self.append_target_workload_typed_archive_load_debug_output(&mut output, context);
        self.append_workload_materialization_mode_summary_debug_output(&mut output, context);
        self.append_workload_existence_timing_summary_debug_output(&mut output, context);
        self.append_workload_typed_indexed_comparison_summary_debug_output(&mut output, context);
        self.append_workload_typed_archive_load_summary_debug_output(&mut output, context);
        self.append_debug_suite_terminal_output(&mut output, context, result_bundle);

        output
    }
}
