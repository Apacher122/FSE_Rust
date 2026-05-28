//! Deterministic range workload generation.

use crate::math::Scalar;
use crate::query::QueryRegion;

use super::case::QueryWorkloadCase;

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
            // Keep generation deterministic so benchmark output is stable.
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
