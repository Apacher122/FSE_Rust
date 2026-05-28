//! Structure and traversal-pressure debug rendering.

use super::super::BenchmarkApplicationRenderer;
use crate::benchmark::math::f64_ratio_or_zero;
use crate::build::sibling_overlap_metrics;

use super::super::super::context::BenchmarkApplicationContext;
use super::super::super::result_bundle::BenchmarkApplicationResultBundle;

impl BenchmarkApplicationRenderer {
    pub(super) fn append_sibling_overlap_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        let metrics = sibling_overlap_metrics(&context.index);

        output.push_str("Sibling overlap metrics\n");
        output.push_str("-----------------------\n");
        output.push_str(&format!("sibling pairs: {}\n", metrics.sibling_pair_count));
        output.push_str(&format!(
            "overlapping sibling pairs: {}\n",
            metrics.overlapping_sibling_pair_count
        ));
        output.push_str(&format!(
            "total overlap extent: {:.2}\n",
            metrics.total_overlap_extent
        ));
        output.push_str(&format!(
            "average overlap extent: {:.2}\n",
            metrics.average_overlap_extent
        ));
        output.push_str(&format!("has overlap: {}\n", metrics.has_overlap()));
        output.push('\n');
    }

    pub(super) fn append_traversal_pressure_debug_output(
        &self,
        output: &mut String,
        result_bundle: &BenchmarkApplicationResultBundle,
    ) {
        let Some(first_baseline_report) = result_bundle.report.baseline_reports.first() else {
            return;
        };

        let aggregate = &first_baseline_report.report.aggregate;

        let visited_per_retained_leaf = f64_ratio_or_zero(
            aggregate.total_fse_visited_nodes,
            aggregate.total_fse_retained_leaves,
        );
        let visited_per_reconstructed_record = f64_ratio_or_zero(
            aggregate.total_fse_visited_nodes,
            aggregate.total_fse_reconstructed_records,
        );

        output.push_str("Traversal pressure summary\n");
        output.push_str("--------------------------\n");
        output.push_str(&format!(
            "total visited nodes: {}\n",
            aggregate.total_fse_visited_nodes
        ));
        output.push_str(&format!(
            "total retained leaves: {}\n",
            aggregate.total_fse_retained_leaves
        ));
        output.push_str(&format!(
            "total reconstructed records: {}\n",
            aggregate.total_fse_reconstructed_records
        ));
        output.push_str(&format!(
            "visited nodes per retained leaf: {:.2}\n",
            visited_per_retained_leaf
        ));
        output.push_str(&format!(
            "visited nodes per reconstructed record: {:.2}\n",
            visited_per_reconstructed_record
        ));
        output.push('\n');
    }
}
