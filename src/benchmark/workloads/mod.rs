//! Benchmark datasets and query workloads.
//!
//! This module contains deterministic datasets and repeatable query workload
//! generators used by benchmark runners and tests.

pub mod datasets;
pub mod workload;

pub use datasets::{
    ClusteredDatasetConfig, clustered_points_2d, generate_clustered_points_2d,
    large_clustered_points_2d,
};

pub use workload::{
    QueryWorkloadCase, RangeWorkloadConfig, clustered_workload_cases,
    generate_range_workload_cases, large_clustered_workload_cases,
};
