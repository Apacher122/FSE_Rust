//! Target workload debug rendering for benchmark application output.
//!
//! This module keeps boundary-workload diagnostics separate from the general
//! benchmark terminal renderer. The diagnostics are intentionally debug-only and
//! should not change query execution behavior.

use std::path::Path;

use super::context::BenchmarkApplicationContext;
use crate::benchmark::reports::TypedArchiveLoadTimingError;

mod candidates;
mod existence_timing;
mod formatting;
mod materialization_modes;
mod reconstruction_timing;
mod resultless_timing;
mod retained_allocation;
mod retained_execution_phase;
mod retained_leaf;
mod stage_timing;
mod target;
mod typed_archive_load;
mod typed_comparison;
mod typed_workload;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TypedQueryIndexArchiveArtifactValidation {
    pub(super) workloads_validated: usize,
    pub(super) matched_records: usize,
}

pub(super) fn write_validated_typed_query_index_archive_artifact<P>(
    context: &BenchmarkApplicationContext,
    path: P,
) -> Result<TypedQueryIndexArchiveArtifactValidation, TypedArchiveLoadTimingError>
where
    P: AsRef<Path>,
{
    typed_archive_load::write_validated_typed_query_index_archive_artifact(context, path)
}
