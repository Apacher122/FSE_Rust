//! Reference-result retained-leaf matching.
//!
//! This module separates covered-leaf reference materialization, partial-leaf
//! exact matching, and retained-leaf orchestration while preserving the
//! reference-result execution API.

mod covered;
mod partial;
mod retained;

pub(super) use self::covered::append_fully_covered_index_references;
pub(super) use self::retained::append_retained_reference_matches;
