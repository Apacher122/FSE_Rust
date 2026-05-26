//! Target workload selection helpers.

use super::super::context::BenchmarkApplicationContext;
use crate::benchmark::BenchmarkDatasetKind;
use crate::benchmark::workloads::QueryWorkloadCase;

const SMALL_TARGET_BOUNDARY_WORKLOAD_NAME: &str = "cluster_boundary_range";
const LARGE_TARGET_BOUNDARY_WORKLOAD_NAME: &str = "large_cross_cluster_boundary";

pub(super) fn target_boundary_workload_name(context: &BenchmarkApplicationContext) -> &'static str {
    match context.suite_config.dataset_kind {
        BenchmarkDatasetKind::SmallClustered2D => SMALL_TARGET_BOUNDARY_WORKLOAD_NAME,
        BenchmarkDatasetKind::LargeClustered2D => LARGE_TARGET_BOUNDARY_WORKLOAD_NAME,
    }
}

/// Appends a target workload debug section with shared header and lookup handling.
///
/// # Runtime Role
///
/// Target workload diagnostics all render the same section header, resolve the
/// dataset-specific boundary workload, print the selected workload name, and
/// handle the same missing-workload fallback. This helper keeps that scaffolding
/// in one place so each diagnostic module only contains its own measurement or
/// rendering logic.
pub(super) fn append_target_workload_debug_section<F>(
    output: &mut String,
    context: &BenchmarkApplicationContext,
    title: &str,
    render: F,
) where
    F: FnOnce(&mut String, &BenchmarkApplicationContext, &QueryWorkloadCase),
{
    output.push_str(title);
    output.push('\n');
    output.push_str(&"-".repeat(title.len()));
    output.push('\n');

    let target_workload_name = target_boundary_workload_name(context);

    let Some(workload) = context
        .workloads
        .iter()
        .find(|workload| workload.name == target_workload_name)
    else {
        output.push_str(&format!("workload: {}\n", target_workload_name));
        output.push_str("status: workload not found\n\n");
        return;
    };

    output.push_str(&format!("workload: {}\n", workload.name));

    render(output, context, workload);

    output.push('\n');
}
