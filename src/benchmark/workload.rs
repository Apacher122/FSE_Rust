//! Reusable benchmark query workloads.

use crate::math::Scalar;
use crate::query::QueryRegion;

/// Named query case used for repeatable benchmark and demo execution.
///
/// # Runtime Role
///
/// `QueryWorkloadCase` gives examples and benchmark code a stable way to run
/// multiple query shapes against the same dataset.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryWorkloadCase {
    /// Human-readable workload name.
    pub name: String,

    /// Query region executed for this workload case.
    pub query: QueryRegion,
}

impl QueryWorkloadCase {
    /// Creates a named workload case.
    pub fn new(name: impl Into<String>, query: QueryRegion) -> Self {
        Self {
            name: name.into(),
            query,
        }
    }
}

/// Configuration for deterministic range workload generation.
///
/// # Runtime Role
///
/// `RangeWorkloadConfig` defines a repeatable family of axis-aligned query
/// regions. It is intended for benchmark workloads where reproducibility matters
/// more than random query diversity.
#[derive(Clone, Debug, PartialEq)]
pub struct RangeWorkloadConfig {
    /// Prefix used for generated workload names.
    pub name_prefix: String,

    /// Number of query regions to generate.
    pub count: usize,

    /// Lower coordinate bound used as the starting point for the first query.
    pub start: Vec<Scalar>,

    /// Per-query shift applied to each subsequent query.
    pub step: Vec<Scalar>,

    /// Width of each generated query region per dimension.
    pub width: Vec<Scalar>,
}

impl RangeWorkloadConfig {
    /// Creates a deterministic range workload configuration.
    ///
    /// # Panics
    ///
    /// Panics when dimensionality is inconsistent or when count is zero.
    pub fn new(
        name_prefix: impl Into<String>,
        count: usize,
        start: Vec<Scalar>,
        step: Vec<Scalar>,
        width: Vec<Scalar>,
    ) -> Self {
        assert!(count > 0, "range workload count must be greater than zero");
        assert!(
            !start.is_empty(),
            "range workload must have at least one dimension"
        );

        assert_eq!(
            start.len(),
            step.len(),
            "range workload start and step dimensionality must match"
        );

        assert_eq!(
            start.len(),
            width.len(),
            "range workload start and width dimensionality must match"
        );

        Self {
            name_prefix: name_prefix.into(),
            count,
            start,
            step,
            width,
        }
    }

    /// Returns the dimensionality of generated queries.
    pub fn dimensions(&self) -> usize {
        self.start.len()
    }
}

/// Generates deterministic axis-aligned range workload cases.
///
/// # Runtime Role
///
/// This function creates a stable sequence of query regions by shifting the
/// lower bound by `step` for each generated case and applying `width` to compute
/// the upper bound.
///
/// # Example Shape
///
/// With `start = [0, 0]`, `step = [10, 10]`, and `width = [5, 5]`, the
/// generated query ranges are:
///
/// - `[0, 0]` to `[5, 5]`
/// - `[10, 10]` to `[15, 15]`
/// - `[20, 20]` to `[25, 25]`
pub fn generate_range_workload_cases(config: &RangeWorkloadConfig) -> Vec<QueryWorkloadCase> {
    let mut workloads = Vec::with_capacity(config.count);

    for index in 0..config.count {
        let mut min = Vec::with_capacity(config.dimensions());
        let mut max = Vec::with_capacity(config.dimensions());

        for dimension in 0..config.dimensions() {
            // generation should be deterministic for stable results
            let lower = config.start[dimension] + config.step[dimension] * index as Scalar;
            let upper = lower + config.width[dimension];

            min.push(lower);
            max.push(upper);
        }

        workloads.push(QueryWorkloadCase::new(
            format!("{}_{:03}", config.name_prefix, index),
            QueryRegion::new(min, max),
        ));
    }

    workloads
}

/// Returns reusable query cases for the deterministic clustered 2D dataset.
///
/// # Runtime Role
///
/// These cases cover different selectivity profiles:
///
/// - selective cluster query
/// - empty query
/// - full-range query
/// - boundary-crossing query
pub fn clustered_workload_cases() -> Vec<QueryWorkloadCase> {
    let mut workloads = generate_range_workload_cases(&RangeWorkloadConfig::new(
        "cluster_range",
        3,
        vec![0.0, 0.0],
        vec![50.0, 50.0],
        vec![5.0, 5.0],
    ));

    // keep a few named cases because they are easier to read in demo output
    workloads.push(QueryWorkloadCase::new(
        "empty_far_range",
        QueryRegion::new(vec![200.0, 200.0], vec![220.0, 220.0]),
    ));

    workloads.push(QueryWorkloadCase::new(
        "full_dataset_range",
        QueryRegion::new(vec![-10.0, -10.0], vec![130.0, 130.0]),
    ));

    workloads.push(QueryWorkloadCase::new(
        "cluster_boundary_range",
        QueryRegion::new(vec![18.0, 18.0], vec![52.0, 52.0]),
    ));

    workloads
}
