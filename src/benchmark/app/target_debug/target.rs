//! Target workload selection and debug rendering helpers.

use std::fmt::{Display, Write};
use std::time::Duration;

use super::super::context::BenchmarkApplicationContext;
use super::candidates::RetainedCandidateBreakdown;
use crate::benchmark::BenchmarkDatasetKind;
use crate::benchmark::reports::output::format_duration_ascii;
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
    writeln!(output, "{title}").expect("writing to String should not fail");
    writeln!(output, "{}", "-".repeat(title.len())).expect("writing to String should not fail");

    let target_workload_name = target_boundary_workload_name(context);

    let Some(workload) = context
        .workloads
        .iter()
        .find(|workload| workload.name == target_workload_name)
    else {
        append_debug_line(output, "workload", target_workload_name);
        output.push_str("status: workload not found\n\n");
        return;
    };

    append_debug_line(output, "workload", &workload.name);

    render(output, context, workload);

    output.push('\n');
}

/// Appends a formatted `label: value` debug line.
///
/// # Runtime Role
///
/// Target workload diagnostics render many scalar facts with the same terminal
/// shape. This helper keeps that output shape consistent without allocating a
/// temporary formatted line before appending to the shared output buffer.
pub(super) fn append_debug_line<T>(output: &mut String, label: &str, value: T)
where
    T: Display,
{
    writeln!(output, "{label}: {value}").expect("writing to String should not fail");
}

/// Appends a duration debug line using the benchmark terminal duration format.
pub(super) fn append_debug_duration_line(output: &mut String, label: &str, duration: Duration) {
    append_debug_line(output, label, format_duration_ascii(duration));
}

/// Appends retained-candidate coverage details in the standard debug order.
///
/// # Runtime Role
///
/// Several target workload diagnostics report the same retained candidate
/// breakdown after running different timing probes. Centralizing the rendering
/// keeps those sections aligned while preserving each diagnostic's measurement
/// logic.
pub(super) fn append_retained_candidate_breakdown(
    output: &mut String,
    breakdown: &RetainedCandidateBreakdown,
) {
    append_debug_line(output, "covered leaves", breakdown.covered_leaves);
    append_debug_line(output, "partial leaves", breakdown.partial_leaves);
    append_debug_line(output, "covered records", breakdown.covered_records);
    append_debug_line(output, "partial records", breakdown.partial_records);
}
