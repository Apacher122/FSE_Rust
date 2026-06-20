//! Typed query view over a base index and pending appended records.
//!
//! This module supports query execution while appended records are waiting for
//! the next archive maintenance rebuild.

use crate::data::{FSERecord, FSERecordBatch, FSERecordBatchError, RowId};
use crate::math::Scalar;
use crate::query::execution::{QueryCountReport, QueryExecutionStats, QueryExistenceReport};

use super::evaluator::evaluate_typed_predicate;
use super::execution::{
    IndexedTypedQueryError, IndexedTypedQueryReport, IndexedTypedQueryRowReport,
    TypedQueryResultRow, count_typed_query_matches, evaluate_typed_query_plan,
    evaluate_typed_query_plan_rows, typed_query_has_match,
};
use super::index::TypedQueryIndex;
use super::plan::TypedQueryPlan;
use super::tombstone::TypedRowTombstoneSet;

/// Borrowed query view over an indexed base batch and an appended record batch.
///
/// # Result Order
///
/// Query methods return base-index matches first, followed by appended batch
/// matches in appended batch order.
#[derive(Clone, Debug)]
pub struct TypedAppendDeltaQueryView<'a> {
    base: &'a TypedQueryIndex,
    appended: &'a FSERecordBatch,
}

impl<'a> TypedAppendDeltaQueryView<'a> {
    /// Creates a typed append delta query view.
    pub fn try_new(
        base: &'a TypedQueryIndex,
        appended: &'a FSERecordBatch,
    ) -> Result<Self, FSERecordBatchError> {
        validate_append_delta(base.batch(), appended)?;

        Ok(Self { base, appended })
    }

    /// Returns the indexed base query data.
    pub fn base(&self) -> &TypedQueryIndex {
        self.base
    }

    /// Returns the appended query data.
    pub fn appended(&self) -> &FSERecordBatch {
        self.appended
    }

    /// Evaluates a typed query plan and returns matching row identifiers.
    pub fn query_row_ids(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<Vec<RowId>, IndexedTypedQueryError> {
        let mut row_ids = self.base.query_row_ids(plan)?;
        row_ids.extend(evaluate_typed_query_plan(self.appended, plan));

        Ok(row_ids)
    }

    /// Evaluates a typed query plan and returns matching row identifiers with
    /// execution statistics.
    pub fn query_row_ids_with_stats(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<IndexedTypedQueryReport, IndexedTypedQueryError> {
        let base_report = self.base.query_row_ids_with_stats(plan)?;
        let appended_row_ids = appended_row_ids_with_tombstone_filter(self.appended, plan, None);
        let stats = append_delta_stats(
            base_report.stats,
            self.appended.len(),
            appended_scan_count(self.appended, plan),
            appended_row_ids.len(),
        );
        let mut row_ids = base_report.row_ids;
        row_ids.extend(appended_row_ids);

        Ok(IndexedTypedQueryReport { row_ids, stats })
    }

    /// Evaluates a typed query plan and excludes tombstoned row identifiers.
    pub fn query_row_ids_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<Vec<RowId>, IndexedTypedQueryError> {
        let mut row_ids = self
            .base
            .query_row_ids_excluding_tombstones(plan, tombstones)?;
        row_ids.extend(appended_row_ids_excluding_tombstones(
            self.appended,
            plan,
            tombstones,
        ));

        Ok(row_ids)
    }

    /// Evaluates a typed query plan with execution statistics and excludes
    /// tombstoned row identifiers.
    pub fn query_row_ids_with_stats_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<IndexedTypedQueryReport, IndexedTypedQueryError> {
        let base_report = self
            .base
            .query_row_ids_with_stats_excluding_tombstones(plan, tombstones)?;
        let appended_row_ids =
            appended_row_ids_with_tombstone_filter(self.appended, plan, Some(tombstones));
        let stats = append_delta_stats(
            base_report.stats,
            self.appended.len(),
            appended_scan_count(self.appended, plan),
            appended_row_ids.len(),
        );
        let mut row_ids = base_report.row_ids;
        row_ids.extend(appended_row_ids);

        Ok(IndexedTypedQueryReport { row_ids, stats })
    }

    /// Evaluates a typed query plan and returns matching typed rows.
    pub fn query_rows(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<Vec<TypedQueryResultRow>, IndexedTypedQueryError> {
        let mut rows = self.base.query_rows(plan)?;
        rows.extend(evaluate_typed_query_plan_rows(self.appended, plan));

        Ok(rows)
    }

    /// Evaluates a typed query plan and returns matching typed rows with
    /// execution statistics.
    pub fn query_rows_with_stats(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<IndexedTypedQueryRowReport, IndexedTypedQueryError> {
        let base_report = self.base.query_rows_with_stats(plan)?;
        let appended_rows = appended_rows_with_tombstone_filter(self.appended, plan, None);
        let stats = append_delta_stats(
            base_report.stats,
            self.appended.len(),
            appended_scan_count(self.appended, plan),
            appended_rows.len(),
        );
        let mut rows = base_report.rows;
        rows.extend(appended_rows);

        Ok(IndexedTypedQueryRowReport { rows, stats })
    }

    /// Evaluates a typed query plan, returns rows, and excludes tombstoned row
    /// identifiers.
    pub fn query_rows_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<Vec<TypedQueryResultRow>, IndexedTypedQueryError> {
        let mut rows = self
            .base
            .query_rows_excluding_tombstones(plan, tombstones)?;
        rows.extend(appended_rows_excluding_tombstones(
            self.appended,
            plan,
            tombstones,
        ));

        Ok(rows)
    }

    /// Evaluates a typed query plan, returns rows with execution statistics,
    /// and excludes tombstoned row identifiers.
    pub fn query_rows_with_stats_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<IndexedTypedQueryRowReport, IndexedTypedQueryError> {
        let base_report = self
            .base
            .query_rows_with_stats_excluding_tombstones(plan, tombstones)?;
        let appended_rows =
            appended_rows_with_tombstone_filter(self.appended, plan, Some(tombstones));
        let stats = append_delta_stats(
            base_report.stats,
            self.appended.len(),
            appended_scan_count(self.appended, plan),
            appended_rows.len(),
        );
        let mut rows = base_report.rows;
        rows.extend(appended_rows);

        Ok(IndexedTypedQueryRowReport { rows, stats })
    }

    /// Counts records that satisfy a typed query plan.
    pub fn count_matches(&self, plan: &TypedQueryPlan) -> Result<usize, IndexedTypedQueryError> {
        Ok(self.base.count_matches(plan)? + count_typed_query_matches(self.appended, plan))
    }

    /// Counts records that satisfy a typed query plan and returns execution
    /// statistics.
    pub fn count_matches_with_stats(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<QueryCountReport, IndexedTypedQueryError> {
        let base_report = self.base.count_matches_with_stats(plan)?;
        let appended_matches = appended_count_with_tombstone_filter(self.appended, plan, None);
        let stats = append_delta_stats(
            base_report.stats,
            self.appended.len(),
            appended_scan_count(self.appended, plan),
            appended_matches,
        );

        Ok(QueryCountReport {
            matched_records: base_report.matched_records + appended_matches,
            stats,
        })
    }

    /// Counts records that satisfy a typed query plan while excluding
    /// tombstoned row identifiers.
    pub fn count_matches_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<usize, IndexedTypedQueryError> {
        Ok(self
            .base
            .count_matches_excluding_tombstones(plan, tombstones)?
            + appended_row_ids_excluding_tombstones(self.appended, plan, tombstones).len())
    }

    /// Counts records that satisfy a typed query plan with execution statistics
    /// while excluding tombstoned row identifiers.
    pub fn count_matches_with_stats_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<QueryCountReport, IndexedTypedQueryError> {
        let base_report = self
            .base
            .count_matches_with_stats_excluding_tombstones(plan, tombstones)?;
        let appended_matches =
            appended_count_with_tombstone_filter(self.appended, plan, Some(tombstones));
        let stats = append_delta_stats(
            base_report.stats,
            self.appended.len(),
            appended_scan_count(self.appended, plan),
            appended_matches,
        );

        Ok(QueryCountReport {
            matched_records: base_report.matched_records + appended_matches,
            stats,
        })
    }

    /// Returns true when a typed query plan matches at least one record.
    pub fn has_match(&self, plan: &TypedQueryPlan) -> Result<bool, IndexedTypedQueryError> {
        if self.base.has_match(plan)? {
            return Ok(true);
        }

        Ok(typed_query_has_match(self.appended, plan))
    }

    /// Returns existence query output with execution statistics.
    pub fn has_match_with_stats(
        &self,
        plan: &TypedQueryPlan,
    ) -> Result<QueryExistenceReport, IndexedTypedQueryError> {
        let base_report = self.base.has_match_with_stats(plan)?;

        if base_report.has_match {
            let inspected_records = base_report.inspected_records;
            let stats = append_delta_stats(base_report.stats, self.appended.len(), 0, 0);

            return Ok(QueryExistenceReport {
                has_match: true,
                inspected_records,
                stats,
            });
        }

        let appended_report = appended_existence_with_tombstone_filter(self.appended, plan, None);
        let base_inspected_records = base_report.inspected_records;
        let stats = append_delta_stats(
            base_report.stats,
            self.appended.len(),
            appended_report.inspected_records,
            usize::from(appended_report.has_match),
        );

        Ok(QueryExistenceReport {
            has_match: appended_report.has_match,
            inspected_records: base_inspected_records + appended_report.inspected_records,
            stats,
        })
    }

    /// Returns true when a typed query plan matches at least one
    /// non-tombstoned record.
    pub fn has_match_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<bool, IndexedTypedQueryError> {
        if self.base.has_match_excluding_tombstones(plan, tombstones)? {
            return Ok(true);
        }

        Ok(
            appended_row_ids_excluding_tombstones(self.appended, plan, tombstones)
                .first()
                .is_some(),
        )
    }

    /// Returns existence query output with execution statistics while excluding
    /// tombstoned row identifiers.
    pub fn has_match_with_stats_excluding_tombstones(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
    ) -> Result<QueryExistenceReport, IndexedTypedQueryError> {
        let base_report = self
            .base
            .has_match_with_stats_excluding_tombstones(plan, tombstones)?;

        if base_report.has_match {
            let inspected_records = base_report.inspected_records;
            let stats = append_delta_stats(base_report.stats, self.appended.len(), 0, 0);

            return Ok(QueryExistenceReport {
                has_match: true,
                inspected_records,
                stats,
            });
        }

        let appended_report =
            appended_existence_with_tombstone_filter(self.appended, plan, Some(tombstones));
        let base_inspected_records = base_report.inspected_records;
        let stats = append_delta_stats(
            base_report.stats,
            self.appended.len(),
            appended_report.inspected_records,
            usize::from(appended_report.has_match),
        );

        Ok(QueryExistenceReport {
            has_match: appended_report.has_match,
            inspected_records: base_inspected_records + appended_report.inspected_records,
            stats,
        })
    }

    /// Visits matching row identifiers for a typed query plan.
    pub fn visit_row_ids<F>(
        &self,
        plan: &TypedQueryPlan,
        mut visitor: F,
    ) -> Result<(), IndexedTypedQueryError>
    where
        F: FnMut(RowId),
    {
        self.base.visit_row_ids(plan, |row_id| {
            visitor(row_id);
        })?;

        for row_id in evaluate_typed_query_plan(self.appended, plan) {
            visitor(row_id);
        }

        Ok(())
    }

    /// Visits matching row identifiers for a typed query plan while excluding
    /// tombstoned row identifiers.
    pub fn visit_row_ids_excluding_tombstones<F>(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
        mut visitor: F,
    ) -> Result<(), IndexedTypedQueryError>
    where
        F: FnMut(RowId),
    {
        self.base
            .visit_row_ids_excluding_tombstones(plan, tombstones, |row_id| {
                visitor(row_id);
            })?;

        for row_id in appended_row_ids_excluding_tombstones(self.appended, plan, tombstones) {
            visitor(row_id);
        }

        Ok(())
    }

    /// Visits matching typed records for a typed query plan.
    pub fn visit_rows<F>(
        &self,
        plan: &TypedQueryPlan,
        mut visitor: F,
    ) -> Result<(), IndexedTypedQueryError>
    where
        F: FnMut(RowId, &FSERecord),
    {
        self.base.visit_rows(plan, |row_id, record| {
            visitor(row_id, record);
        })?;

        for row_id in evaluate_typed_query_plan(self.appended, plan) {
            if let Some(record) = self.appended.record_for_row_id(row_id) {
                visitor(row_id, record);
            }
        }

        Ok(())
    }

    /// Visits matching typed records for a typed query plan while excluding
    /// tombstoned row identifiers.
    pub fn visit_rows_excluding_tombstones<F>(
        &self,
        plan: &TypedQueryPlan,
        tombstones: &TypedRowTombstoneSet,
        mut visitor: F,
    ) -> Result<(), IndexedTypedQueryError>
    where
        F: FnMut(RowId, &FSERecord),
    {
        self.base
            .visit_rows_excluding_tombstones(plan, tombstones, |row_id, record| {
                visitor(row_id, record);
            })?;

        for row_id in appended_row_ids_excluding_tombstones(self.appended, plan, tombstones) {
            if let Some(record) = self.appended.record_for_row_id(row_id) {
                visitor(row_id, record);
            }
        }

        Ok(())
    }
}

fn validate_append_delta(
    base: &FSERecordBatch,
    appended: &FSERecordBatch,
) -> Result<(), FSERecordBatchError> {
    if base.schema() != appended.schema() {
        return Err(FSERecordBatchError::SchemaMismatch);
    }

    if appended.is_empty() {
        return Err(FSERecordBatchError::EmptyAppendBatch);
    }

    for row_id in appended.row_ids() {
        if base.row_index_for_row_id(*row_id).is_some() {
            return Err(FSERecordBatchError::DuplicateRowId { row_id: *row_id });
        }
    }

    Ok(())
}

fn appended_row_ids_with_tombstone_filter(
    appended: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: Option<&TypedRowTombstoneSet>,
) -> Vec<RowId> {
    if plan.is_unsatisfiable() {
        return Vec::new();
    }

    appended
        .row_ids()
        .iter()
        .zip(appended.records())
        .filter_map(|(row_id, record)| {
            if tombstones.is_some_and(|tombstones| tombstones.contains(*row_id)) {
                return None;
            }

            record_matches_plan(record, plan).then_some(*row_id)
        })
        .collect()
}

fn appended_rows_with_tombstone_filter(
    appended: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: Option<&TypedRowTombstoneSet>,
) -> Vec<TypedQueryResultRow> {
    if plan.is_unsatisfiable() {
        return Vec::new();
    }

    appended
        .row_ids()
        .iter()
        .zip(appended.records())
        .filter_map(|(row_id, record)| {
            if tombstones.is_some_and(|tombstones| tombstones.contains(*row_id)) {
                return None;
            }

            record_matches_plan(record, plan)
                .then(|| TypedQueryResultRow::new(*row_id, record.clone()))
        })
        .collect()
}

fn appended_count_with_tombstone_filter(
    appended: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: Option<&TypedRowTombstoneSet>,
) -> usize {
    appended_row_ids_with_tombstone_filter(appended, plan, tombstones).len()
}

fn appended_existence_with_tombstone_filter(
    appended: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: Option<&TypedRowTombstoneSet>,
) -> QueryExistenceReport {
    if plan.is_unsatisfiable() {
        return QueryExistenceReport {
            has_match: false,
            inspected_records: 0,
            stats: QueryExecutionStats::default(),
        };
    }

    let mut inspected_records = 0;

    for (row_id, record) in appended.row_ids().iter().zip(appended.records()) {
        inspected_records += 1;

        if tombstones.is_some_and(|tombstones| tombstones.contains(*row_id)) {
            continue;
        }

        if record_matches_plan(record, plan) {
            return QueryExistenceReport {
                has_match: true,
                inspected_records,
                stats: QueryExecutionStats::default(),
            };
        }
    }

    QueryExistenceReport {
        has_match: false,
        inspected_records,
        stats: QueryExecutionStats::default(),
    }
}

fn appended_row_ids_excluding_tombstones(
    appended: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Vec<RowId> {
    appended_row_ids_with_tombstone_filter(appended, plan, Some(tombstones))
}

fn appended_rows_excluding_tombstones(
    appended: &FSERecordBatch,
    plan: &TypedQueryPlan,
    tombstones: &TypedRowTombstoneSet,
) -> Vec<TypedQueryResultRow> {
    appended_rows_with_tombstone_filter(appended, plan, Some(tombstones))
}

fn appended_scan_count(appended: &FSERecordBatch, plan: &TypedQueryPlan) -> usize {
    if plan.is_unsatisfiable() {
        return 0;
    }

    appended.len()
}

fn append_delta_stats(
    mut base_stats: QueryExecutionStats,
    appended_total_records: usize,
    appended_reconstructed_records: usize,
    appended_matched_records: usize,
) -> QueryExecutionStats {
    base_stats.total_records += appended_total_records;
    base_stats.reconstructed_records += appended_reconstructed_records;
    base_stats.matched_records += appended_matched_records;
    base_stats.candidate_ratio =
        candidate_ratio(base_stats.reconstructed_records, base_stats.total_records);

    base_stats
}

fn record_matches_plan(record: &FSERecord, plan: &TypedQueryPlan) -> bool {
    plan.predicates()
        .iter()
        .all(|predicate| evaluate_typed_predicate(record, predicate))
}

fn candidate_ratio(reconstructed_records: usize, total_records: usize) -> Scalar {
    if total_records == 0 {
        return 0.0;
    }

    reconstructed_records as Scalar / total_records as Scalar
}
