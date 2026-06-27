//! Typed query comparison reports.

use crate::benchmark::math::scalar_ratio_or_zero;
use crate::benchmark::reports::timing::{
    RepeatedComparisonTimingReport, RepeatedTimingConfig, TimingReport, duration_ratio,
    measure_elapsed, measure_repeated_comparison_interleaved,
};
use crate::data::RowId;
use crate::math::Scalar;
use crate::query::{
    IndexedTypedQueryError, QueryExecutionStats, TypedQueryIndex, TypedQueryOutputContract,
    TypedQueryPlan, TypedQueryPlanningDiagnostics, evaluate_typed_query_plan,
    plan_typed_query_execution,
};

/// Comparison report for typed batch scan and indexed typed execution.
///
/// # Runtime Role
///
/// `TypedQueryComparisonReport` records exactness, timing, and retained-work
/// metrics for a typed query plan executed through a typed query index.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedQueryComparisonReport {
    /// Number of records matched by typed batch scan.
    pub baseline_matched_records: usize,

    /// Number of records matched by indexed typed execution.
    pub indexed_matched_records: usize,

    /// Geometric execution statistics for indexed typed execution.
    pub indexed_stats: QueryExecutionStats,

    /// Planning diagnostics for the indexed typed execution path.
    pub planning_diagnostics: TypedQueryPlanningDiagnostics,

    /// Wall-clock timing measurements for one execution of both paths.
    pub timing: TimingReport,

    /// Repeated timing measurements for typed batch scan and indexed typed execution.
    pub repeated_timing: RepeatedComparisonTimingReport,

    /// Single-run timing ratio computed as baseline elapsed divided by indexed elapsed.
    pub single_run_timing_ratio: f64,

    /// Average timing ratio computed as baseline average elapsed divided by indexed average elapsed.
    pub average_timing_ratio: f64,

    /// Number of baseline record evaluations avoided by indexed execution.
    pub avoided_record_evaluations: usize,

    /// Fraction of baseline record evaluations avoided by indexed execution.
    pub record_evaluation_avoidance_ratio: Scalar,

    /// Fraction of total records reconstructed by indexed execution.
    pub candidate_ratio: Scalar,

    /// Fraction of leaf partitions retained by indexed execution.
    pub retained_leaf_ratio: Scalar,
}

/// Compares indexed typed execution against typed batch scan.
///
/// # Runtime Role
///
/// This function provides the default typed comparison used before CSV or CLI
/// benchmark integration. It uses the default repeated timing configuration.
pub fn compare_typed_query_execution(
    query_index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
) -> Result<TypedQueryComparisonReport, IndexedTypedQueryError> {
    compare_typed_query_execution_repeated(query_index, plan, &RepeatedTimingConfig::default())
}

/// Compares indexed typed execution against typed batch scan with repeated timing.
///
/// # Panics
///
/// Panics when indexed typed execution returns a different exact row-id set
/// than typed batch scan.
pub fn compare_typed_query_execution_repeated(
    query_index: &TypedQueryIndex,
    plan: &TypedQueryPlan,
    timing_config: &RepeatedTimingConfig,
) -> Result<TypedQueryComparisonReport, IndexedTypedQueryError> {
    let planning_diagnostics =
        plan_typed_query_execution(query_index, plan, TypedQueryOutputContract::RowIds);
    let (baseline_row_ids, baseline_elapsed) = measure_elapsed(|| {
        let row_ids = evaluate_typed_query_plan(query_index.batch(), plan);
        std::hint::black_box(row_ids.len());
        row_ids
    });
    let (indexed_report, indexed_elapsed) = measure_elapsed(|| {
        let report = query_index.query_row_ids_with_planning(plan)?;
        std::hint::black_box(report.row_ids.len());
        Ok::<_, IndexedTypedQueryError>(report)
    });
    let indexed_report = indexed_report?;

    assert_same_row_id_set(&baseline_row_ids, &indexed_report.row_ids);

    let repeated_timing = measure_repeated_comparison_interleaved(
        timing_config,
        || {
            let row_ids = evaluate_typed_query_plan(query_index.batch(), plan);
            std::hint::black_box(row_ids.len());
        },
        || {
            let report = query_index
                .query_row_ids_with_planning(plan)
                .expect("indexed typed query should match the validated single-run comparison");
            std::hint::black_box(report.row_ids.len());
        },
    );

    let evaluated_records = query_index.batch().len();
    let reconstructed_records = indexed_report.execution_stats.reconstructed_records;
    let avoided_record_evaluations = evaluated_records.saturating_sub(reconstructed_records);
    let record_evaluation_avoidance_ratio =
        scalar_ratio_or_zero(avoided_record_evaluations, evaluated_records);

    Ok(TypedQueryComparisonReport {
        baseline_matched_records: baseline_row_ids.len(),
        indexed_matched_records: indexed_report.row_ids.len(),
        indexed_stats: indexed_report.execution_stats.clone(),
        planning_diagnostics,
        timing: TimingReport {
            baseline_elapsed,
            fse_elapsed: indexed_elapsed,
        },
        single_run_timing_ratio: duration_ratio(baseline_elapsed, indexed_elapsed),
        average_timing_ratio: duration_ratio(
            repeated_timing.baseline.average_elapsed,
            repeated_timing.fse.average_elapsed,
        ),
        repeated_timing,
        avoided_record_evaluations,
        record_evaluation_avoidance_ratio,
        candidate_ratio: indexed_report.execution_stats.candidate_ratio,
        retained_leaf_ratio: indexed_report.execution_stats.retained_leaf_ratio,
    })
}

fn assert_same_row_id_set(left: &[RowId], right: &[RowId]) {
    let mut left = left.to_vec();
    let mut right = right.to_vec();

    left.sort_unstable();
    right.sort_unstable();

    assert_eq!(
        left, right,
        "indexed typed query results must match typed batch scan results"
    );
}
