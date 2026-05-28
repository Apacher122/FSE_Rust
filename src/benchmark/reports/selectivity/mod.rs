//! Selectivity-bucketed workload summaries.

mod bucket;
mod render;
mod report;
mod summary;

pub use bucket::SelectivityBucket;
pub use render::render_selectivity_bucketed_workload_summary;
pub use report::{SelectivityBucketSummary, SelectivityBucketedWorkloadSummary};
pub use summary::summarize_workloads_by_selectivity;
