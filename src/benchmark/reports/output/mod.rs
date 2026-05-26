//! Terminal output rendering for benchmark reports.
//!
//! This module owns benchmark terminal report rendering. Overview rendering,
//! suite rendering, multi-baseline rendering, and portable duration formatting
//! are split by responsibility while preserving the existing public API.

mod duration;
mod multi;
mod overview;
mod suite;

pub use duration::format_duration_ascii;
pub use multi::render_multi_baseline_summary;
pub use overview::{BenchmarkRunOverview, render_benchmark_overview};
pub use suite::{render_named_baseline_suite_report, render_suite_report};
