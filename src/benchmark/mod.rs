pub mod comparison;
pub mod datasets;
pub mod scan;
pub mod summary;
pub mod workload;

pub use comparison::{QueryComparisonReport, compare_query_execution};
pub use datasets::clustered_points_2d;
pub use scan::{FlatScanReport, FlatScanStats, flat_scan, flat_scan_with_stats};
pub use summary::{AggregateWorkloadMetrics, aggregate_workload_metrics};
pub use workload::{QueryWorkloadCase, clustered_workload_cases};
