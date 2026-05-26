//! Retained leaf detail diagnostics.

use super::super::context::BenchmarkApplicationContext;
use super::super::renderer::BenchmarkApplicationRenderer;
use super::formatting::{
    format_bounds_max, format_bounds_min, format_coordinate_values, retained_leaf_coverage_label,
};
use super::target::append_target_workload_debug_section;
use crate::query::traverse_with_stats;

impl BenchmarkApplicationRenderer {
    pub(crate) fn append_target_workload_retained_leaf_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        append_target_workload_debug_section(
            output,
            context,
            "Target workload retained leaf details",
            |output, context, workload| {
                let traversal = traverse_with_stats(&context.index, &workload.query);

                output.push_str(&format!(
                    "query min: {}\n",
                    format_coordinate_values(&workload.query.min)
                ));
                output.push_str(&format!(
                    "query max: {}\n",
                    format_coordinate_values(&workload.query.max)
                ));
                output.push_str(&format!(
                    "retained leaves: {}\n",
                    traversal.stats.retained_leaves
                ));
                output.push_str("leaf | coverage | records | bounds min | bounds max | volume\n");

                if traversal.retained_leaves.is_empty() {
                    output.push_str("none\n");
                    return;
                }

                for retained_leaf in &traversal.retained_leaves {
                    let node = &context.index.nodes[retained_leaf.node_id];

                    output.push_str(&format!(
                        "{} | {} | {} | {} | {} | {:.2}\n",
                        retained_leaf.node_id,
                        retained_leaf_coverage_label(retained_leaf.coverage),
                        node.stored_cardinality(),
                        format_bounds_min(&node.bounds),
                        format_bounds_max(&node.bounds),
                        node.bounds.volume(),
                    ));
                }
            },
        );
    }
}
