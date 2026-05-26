//! Target workload debug rendering for benchmark application output.
//!
//! This module keeps boundary-workload diagnostics separate from the general
//! benchmark terminal renderer. The diagnostics are intentionally debug-only and
//! should not change query execution behavior.

mod candidates;
mod count_only;
mod formatting;
mod reconstruction_timing;
mod resultless_timing;
mod retained_allocation;
mod retained_execution_phase;
mod retained_leaf;
mod stage_timing;
mod target;
