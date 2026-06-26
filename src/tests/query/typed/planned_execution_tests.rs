use crate::build::{BuildConfig, FSEBuilder};
use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSESchema,
    FSESchemaDimensionMapping, FSEValue, RowId,
};
use crate::encoding::{
    CategoricalDictionaryEncoder, ComposedRecordEncoder, FloatEncoder, IntegerEncoder,
    TimestampMillisEncoder,
};
use crate::query::{
    FSEPredicate, FSEPredicateField, TypedAppendDeltaQueryView, TypedQueryExecutionStrategy,
    TypedQueryIndex, TypedQueryOutputContract, TypedQueryPlan, TypedQueryPlanBuilder,
    TypedQueryPlanningReason, TypedQueryResultRow, TypedRowTombstoneSet,
    planned_append_delta_query_count_matches, planned_append_delta_query_has_match,
    planned_append_delta_query_row_ids, planned_append_delta_query_row_ids_excluding_tombstones,
    planned_append_delta_query_rows, planned_append_delta_query_visit_row_ids,
    planned_append_delta_query_visit_rows, planned_typed_query_count_matches,
    planned_typed_query_has_match, planned_typed_query_has_match_excluding_tombstones,
    planned_typed_query_row_ids, planned_typed_query_rows, planned_typed_query_visit_row_ids,
    planned_typed_query_visit_rows,
};

#[test]
fn planned_typed_query_row_ids_uses_fse_for_selective_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let plan = score_and_class_plan(&schema, &mapping);

    let report = planned_typed_query_row_ids(&query_index, &plan)
        .expect("planned typed query should execute");

    assert_eq!(report.row_ids, vec![RowId::new(100), RowId::new(103)]);
    assert_eq!(
        report.diagnostics.strategy,
        TypedQueryExecutionStrategy::FseTraversal
    );
    assert_eq!(
        report.diagnostics.reason,
        TypedQueryPlanningReason::SelectiveGeometry
    );
}

#[test]
fn planned_typed_query_row_ids_uses_flat_scan_for_broad_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let plan = score_range_plan(&schema, &mapping, 0.0, 100.0);

    let report = query_index
        .query_row_ids_with_planning(&plan)
        .expect("planned typed query should execute");

    assert_eq!(
        report.row_ids,
        vec![
            RowId::new(100),
            RowId::new(101),
            RowId::new(102),
            RowId::new(103)
        ]
    );
    assert_eq!(
        report.diagnostics.strategy,
        TypedQueryExecutionStrategy::FlatScan
    );
    assert_eq!(
        report.diagnostics.reason,
        TypedQueryPlanningReason::BroadGeometry
    );
}

#[test]
fn planned_typed_query_row_ids_uses_flat_scan_for_broad_categorical_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = broad_class_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let plan = class_equality_plan(&schema, &mapping, "alpha");

    let report = query_index
        .query_row_ids_with_planning(&plan)
        .expect("planned typed query should execute");

    assert_eq!(
        report.row_ids,
        vec![RowId::new(100), RowId::new(101), RowId::new(102)]
    );
    assert_eq!(
        report.diagnostics.strategy,
        TypedQueryExecutionStrategy::FlatScan
    );
    assert_eq!(
        report.diagnostics.reason,
        TypedQueryPlanningReason::BroadGeometry
    );
}

#[test]
fn planned_typed_query_row_ids_returns_noop_for_unsatisfiable_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let low_plan = score_range_plan(&schema, &mapping, 0.0, 1.0);
    let high_plan = score_range_plan(&schema, &mapping, 10.0, 11.0);
    let plan = TypedQueryPlan::conjunctive(vec![low_plan, high_plan])
        .expect("valid plans should produce a conjunction");

    let report = query_index
        .query_row_ids_with_planning(&plan)
        .expect("planned typed query should execute");

    assert!(report.row_ids.is_empty());
    assert_eq!(
        report.diagnostics.strategy,
        TypedQueryExecutionStrategy::NoOp
    );
    assert_eq!(
        report.diagnostics.reason,
        TypedQueryPlanningReason::UnsatisfiablePlan
    );
}

#[test]
fn planned_typed_query_rows_uses_fse_for_selective_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let plan = score_and_class_plan(&schema, &mapping);

    let report = query_index
        .query_rows_with_planning(&plan)
        .expect("planned typed rows should execute");
    let function_report =
        planned_typed_query_rows(&query_index, &plan).expect("planned typed rows should execute");

    assert_eq!(
        result_row_ids(&report.rows),
        vec![RowId::new(100), RowId::new(103)]
    );
    assert_eq!(function_report, report);
    assert_eq!(
        report.diagnostics.output_contract,
        TypedQueryOutputContract::Rows
    );
    assert_eq!(
        report.diagnostics.strategy,
        TypedQueryExecutionStrategy::FseTraversal
    );
    assert_eq!(
        report.diagnostics.reason,
        TypedQueryPlanningReason::SelectiveGeometry
    );
}

#[test]
fn planned_typed_query_visit_row_ids_uses_fse_for_selective_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let plan = score_and_class_plan(&schema, &mapping);
    let mut row_ids = Vec::new();

    let report = query_index
        .visit_row_ids_with_planning(&plan, |row_id| row_ids.push(row_id))
        .expect("planned typed row-id visitor should execute");
    let mut function_row_ids = Vec::new();
    let function_report = planned_typed_query_visit_row_ids(&query_index, &plan, |row_id| {
        function_row_ids.push(row_id);
    })
    .expect("planned typed row-id visitor should execute");

    assert_eq!(row_ids, vec![RowId::new(100), RowId::new(103)]);
    assert_eq!(function_row_ids, row_ids);
    assert_eq!(function_report, report);
    assert_eq!(report.visited_records, 2);
    assert_eq!(
        report.diagnostics.output_contract,
        TypedQueryOutputContract::RowIdVisitor
    );
    assert_eq!(
        report.diagnostics.strategy,
        TypedQueryExecutionStrategy::FseTraversal
    );
    assert_eq!(
        report.diagnostics.reason,
        TypedQueryPlanningReason::SelectiveGeometry
    );
}

#[test]
fn planned_typed_query_visit_rows_uses_fse_for_selective_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let plan = score_and_class_plan(&schema, &mapping);
    let mut row_ids = Vec::new();

    let report = query_index
        .visit_rows_with_planning(&plan, |row_id, _record| row_ids.push(row_id))
        .expect("planned typed row visitor should execute");
    let mut function_row_ids = Vec::new();
    let function_report = planned_typed_query_visit_rows(&query_index, &plan, |row_id, _record| {
        function_row_ids.push(row_id);
    })
    .expect("planned typed row visitor should execute");

    assert_eq!(row_ids, vec![RowId::new(100), RowId::new(103)]);
    assert_eq!(function_row_ids, row_ids);
    assert_eq!(function_report, report);
    assert_eq!(report.visited_records, 2);
    assert_eq!(
        report.diagnostics.output_contract,
        TypedQueryOutputContract::RowVisitor
    );
    assert_eq!(
        report.diagnostics.strategy,
        TypedQueryExecutionStrategy::FseTraversal
    );
    assert_eq!(
        report.diagnostics.reason,
        TypedQueryPlanningReason::SelectiveGeometry
    );
}

#[test]
fn planned_typed_query_excludes_tombstones_from_planned_contracts() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let plan = score_and_class_plan(&schema, &mapping);
    let tombstones = TypedRowTombstoneSet::from_row_ids(vec![RowId::new(100)]);
    let mut visited_row_ids = Vec::new();
    let mut visited_rows = Vec::new();

    let row_id_report = query_index
        .query_row_ids_with_planning_excluding_tombstones(&plan, &tombstones)
        .expect("planned tombstone row-id query should execute");
    let row_report = query_index
        .query_rows_with_planning_excluding_tombstones(&plan, &tombstones)
        .expect("planned tombstone row query should execute");
    let count_report = query_index
        .count_matches_with_planning_excluding_tombstones(&plan, &tombstones)
        .expect("planned tombstone count query should execute");
    let existence_report =
        planned_typed_query_has_match_excluding_tombstones(&query_index, &plan, &tombstones)
            .expect("planned tombstone existence query should execute");
    let row_id_visit_report = query_index
        .visit_row_ids_with_planning_excluding_tombstones(&plan, &tombstones, |row_id| {
            visited_row_ids.push(row_id);
        })
        .expect("planned tombstone row-id visitor should execute");
    let row_visit_report = query_index
        .visit_rows_with_planning_excluding_tombstones(&plan, &tombstones, |row_id, _record| {
            visited_rows.push(row_id);
        })
        .expect("planned tombstone row visitor should execute");

    assert_eq!(row_id_report.row_ids, vec![RowId::new(103)]);
    assert_eq!(result_row_ids(&row_report.rows), vec![RowId::new(103)]);
    assert_eq!(count_report.matched_records, 1);
    assert!(existence_report.has_match);
    assert_eq!(visited_row_ids, vec![RowId::new(103)]);
    assert_eq!(visited_rows, vec![RowId::new(103)]);
    assert_eq!(row_id_visit_report.visited_records, 1);
    assert_eq!(row_visit_report.visited_records, 1);
    assert_eq!(
        row_id_report.diagnostics.strategy,
        TypedQueryExecutionStrategy::FseTraversal
    );
    assert_eq!(
        row_id_report.diagnostics.output_contract,
        TypedQueryOutputContract::RowIds
    );
}

#[test]
fn planned_typed_query_existence_returns_false_when_all_matches_are_tombstoned() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let plan = score_and_class_plan(&schema, &mapping);
    let tombstones = TypedRowTombstoneSet::from_row_ids(vec![RowId::new(100), RowId::new(103)]);

    let report = query_index
        .has_match_with_planning_excluding_tombstones(&plan, &tombstones)
        .expect("planned tombstone existence query should execute");

    assert!(!report.has_match);
    assert_eq!(
        report.diagnostics.output_contract,
        TypedQueryOutputContract::Existence
    );
    assert_eq!(
        report.diagnostics.strategy,
        TypedQueryExecutionStrategy::FseTraversal
    );
}

#[test]
fn planned_typed_query_count_matches_uses_flat_scan_for_broad_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let plan = score_range_plan(&schema, &mapping, 0.0, 100.0);

    let report = query_index
        .count_matches_with_planning(&plan)
        .expect("planned typed count should execute");
    let function_report = planned_typed_query_count_matches(&query_index, &plan)
        .expect("planned typed count should execute");

    assert_eq!(report.matched_records, 4);
    assert_eq!(function_report, report);
    assert_eq!(
        report.diagnostics.output_contract,
        TypedQueryOutputContract::Count
    );
    assert_eq!(
        report.diagnostics.strategy,
        TypedQueryExecutionStrategy::FlatScan
    );
    assert_eq!(
        report.diagnostics.reason,
        TypedQueryPlanningReason::BroadGeometry
    );
}

#[test]
fn planned_typed_query_has_match_returns_noop_for_unsatisfiable_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let low_plan = score_range_plan(&schema, &mapping, 0.0, 1.0);
    let high_plan = score_range_plan(&schema, &mapping, 10.0, 11.0);
    let plan = TypedQueryPlan::conjunctive(vec![low_plan, high_plan])
        .expect("valid plans should produce a conjunction");

    let report = query_index
        .has_match_with_planning(&plan)
        .expect("planned typed existence should execute");
    let function_report = planned_typed_query_has_match(&query_index, &plan)
        .expect("planned typed existence should execute");

    assert!(!report.has_match);
    assert_eq!(function_report, report);
    assert_eq!(
        report.diagnostics.output_contract,
        TypedQueryOutputContract::Existence
    );
    assert_eq!(
        report.diagnostics.strategy,
        TypedQueryExecutionStrategy::NoOp
    );
    assert_eq!(
        report.diagnostics.reason,
        TypedQueryPlanningReason::UnsatisfiablePlan
    );
}

#[test]
fn planned_append_delta_query_row_ids_uses_hybrid_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let view = TypedAppendDeltaQueryView::try_new(&query_index, &appended)
        .expect("valid append batch should produce a query view");
    let plan = score_and_class_plan(&schema, &mapping);

    let report = view
        .query_row_ids_with_planning(&plan)
        .expect("planned append-delta query should execute");
    let function_report = planned_append_delta_query_row_ids(&view, &plan)
        .expect("planned append-delta query should execute");

    assert_eq!(
        report.row_ids,
        vec![RowId::new(100), RowId::new(103), RowId::new(104)]
    );
    assert_eq!(function_report, report);
    assert_eq!(
        report.diagnostics.strategy,
        TypedQueryExecutionStrategy::Hybrid
    );
    assert_eq!(
        report.diagnostics.reason,
        TypedQueryPlanningReason::AppendDeltaScan
    );
}

#[test]
fn planned_append_delta_query_rows_uses_hybrid_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let view = TypedAppendDeltaQueryView::try_new(&query_index, &appended)
        .expect("valid append batch should produce a query view");
    let plan = score_and_class_plan(&schema, &mapping);

    let report = view
        .query_rows_with_planning(&plan)
        .expect("planned append-delta rows should execute");
    let function_report = planned_append_delta_query_rows(&view, &plan)
        .expect("planned append-delta rows should execute");

    assert_eq!(
        result_row_ids(&report.rows),
        vec![RowId::new(100), RowId::new(103), RowId::new(104)]
    );
    assert_eq!(function_report, report);
    assert_eq!(
        report.diagnostics.output_contract,
        TypedQueryOutputContract::Rows
    );
    assert_eq!(
        report.diagnostics.strategy,
        TypedQueryExecutionStrategy::Hybrid
    );
    assert_eq!(
        report.diagnostics.reason,
        TypedQueryPlanningReason::AppendDeltaScan
    );
}

#[test]
fn planned_append_delta_query_visit_row_ids_uses_hybrid_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let view = TypedAppendDeltaQueryView::try_new(&query_index, &appended)
        .expect("valid append batch should produce a query view");
    let plan = score_and_class_plan(&schema, &mapping);
    let mut row_ids = Vec::new();

    let report = view
        .visit_row_ids_with_planning(&plan, |row_id| row_ids.push(row_id))
        .expect("planned append-delta row-id visitor should execute");
    let mut function_row_ids = Vec::new();
    let function_report = planned_append_delta_query_visit_row_ids(&view, &plan, |row_id| {
        function_row_ids.push(row_id);
    })
    .expect("planned append-delta row-id visitor should execute");

    assert_eq!(
        row_ids,
        vec![RowId::new(100), RowId::new(103), RowId::new(104)]
    );
    assert_eq!(function_row_ids, row_ids);
    assert_eq!(function_report, report);
    assert_eq!(report.visited_records, 3);
    assert_eq!(
        report.diagnostics.output_contract,
        TypedQueryOutputContract::RowIdVisitor
    );
    assert_eq!(
        report.diagnostics.strategy,
        TypedQueryExecutionStrategy::Hybrid
    );
    assert_eq!(
        report.diagnostics.reason,
        TypedQueryPlanningReason::AppendDeltaScan
    );
}

#[test]
fn planned_append_delta_query_visit_rows_uses_hybrid_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let view = TypedAppendDeltaQueryView::try_new(&query_index, &appended)
        .expect("valid append batch should produce a query view");
    let plan = score_and_class_plan(&schema, &mapping);
    let mut row_ids = Vec::new();

    let report = view
        .visit_rows_with_planning(&plan, |row_id, _record| row_ids.push(row_id))
        .expect("planned append-delta row visitor should execute");
    let mut function_row_ids = Vec::new();
    let function_report = planned_append_delta_query_visit_rows(&view, &plan, |row_id, _record| {
        function_row_ids.push(row_id);
    })
    .expect("planned append-delta row visitor should execute");

    assert_eq!(
        row_ids,
        vec![RowId::new(100), RowId::new(103), RowId::new(104)]
    );
    assert_eq!(function_row_ids, row_ids);
    assert_eq!(function_report, report);
    assert_eq!(report.visited_records, 3);
    assert_eq!(
        report.diagnostics.output_contract,
        TypedQueryOutputContract::RowVisitor
    );
    assert_eq!(
        report.diagnostics.strategy,
        TypedQueryExecutionStrategy::Hybrid
    );
    assert_eq!(
        report.diagnostics.reason,
        TypedQueryPlanningReason::AppendDeltaScan
    );
}

#[test]
fn planned_append_delta_query_excludes_tombstones_from_planned_contracts() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let view = TypedAppendDeltaQueryView::try_new(&query_index, &appended)
        .expect("valid append batch should produce a query view");
    let plan = score_and_class_plan(&schema, &mapping);
    let tombstones = TypedRowTombstoneSet::from_row_ids(vec![RowId::new(103)]);
    let mut visited_row_ids = Vec::new();
    let mut visited_rows = Vec::new();

    let row_id_report =
        planned_append_delta_query_row_ids_excluding_tombstones(&view, &plan, &tombstones)
            .expect("planned append-delta tombstone row-id query should execute");
    let row_report = view
        .query_rows_with_planning_excluding_tombstones(&plan, &tombstones)
        .expect("planned append-delta tombstone row query should execute");
    let count_report = view
        .count_matches_with_planning_excluding_tombstones(&plan, &tombstones)
        .expect("planned append-delta tombstone count query should execute");
    let existence_report = view
        .has_match_with_planning_excluding_tombstones(&plan, &tombstones)
        .expect("planned append-delta tombstone existence query should execute");
    let row_id_visit_report = view
        .visit_row_ids_with_planning_excluding_tombstones(&plan, &tombstones, |row_id| {
            visited_row_ids.push(row_id);
        })
        .expect("planned append-delta tombstone row-id visitor should execute");
    let row_visit_report = view
        .visit_rows_with_planning_excluding_tombstones(&plan, &tombstones, |row_id, _record| {
            visited_rows.push(row_id);
        })
        .expect("planned append-delta tombstone row visitor should execute");

    assert_eq!(
        row_id_report.row_ids,
        vec![RowId::new(100), RowId::new(104)]
    );
    assert_eq!(
        result_row_ids(&row_report.rows),
        vec![RowId::new(100), RowId::new(104)]
    );
    assert_eq!(count_report.matched_records, 2);
    assert!(existence_report.has_match);
    assert_eq!(visited_row_ids, vec![RowId::new(100), RowId::new(104)]);
    assert_eq!(visited_rows, vec![RowId::new(100), RowId::new(104)]);
    assert_eq!(row_id_visit_report.visited_records, 2);
    assert_eq!(row_visit_report.visited_records, 2);
    assert_eq!(
        row_id_report.diagnostics.strategy,
        TypedQueryExecutionStrategy::Hybrid
    );
    assert_eq!(
        row_id_report.diagnostics.reason,
        TypedQueryPlanningReason::AppendDeltaScan
    );
}

#[test]
fn planned_append_delta_query_count_matches_uses_hybrid_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let view = TypedAppendDeltaQueryView::try_new(&query_index, &appended)
        .expect("valid append batch should produce a query view");
    let plan = score_and_class_plan(&schema, &mapping);

    let report = view
        .count_matches_with_planning(&plan)
        .expect("planned append-delta count should execute");
    let function_report = planned_append_delta_query_count_matches(&view, &plan)
        .expect("planned append-delta count should execute");

    assert_eq!(report.matched_records, 3);
    assert_eq!(function_report, report);
    assert_eq!(
        report.diagnostics.output_contract,
        TypedQueryOutputContract::Count
    );
    assert_eq!(
        report.diagnostics.strategy,
        TypedQueryExecutionStrategy::Hybrid
    );
    assert_eq!(
        report.diagnostics.reason,
        TypedQueryPlanningReason::AppendDeltaScan
    );
}

#[test]
fn planned_append_delta_query_has_match_uses_hybrid_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let view = TypedAppendDeltaQueryView::try_new(&query_index, &appended)
        .expect("valid append batch should produce a query view");
    let plan = score_and_class_plan(&schema, &mapping);

    let report = view
        .has_match_with_planning(&plan)
        .expect("planned append-delta existence should execute");
    let function_report = planned_append_delta_query_has_match(&view, &plan)
        .expect("planned append-delta existence should execute");

    assert!(report.has_match);
    assert_eq!(function_report, report);
    assert_eq!(
        report.diagnostics.output_contract,
        TypedQueryOutputContract::Existence
    );
    assert_eq!(
        report.diagnostics.strategy,
        TypedQueryExecutionStrategy::Hybrid
    );
    assert_eq!(
        report.diagnostics.reason,
        TypedQueryPlanningReason::AppendDeltaScan
    );
}

fn score_and_class_plan(schema: &FSESchema, mapping: &FSESchemaDimensionMapping) -> TypedQueryPlan {
    TypedQueryPlanBuilder::new(schema, mapping)
        .with_categorical_encoder(2, class_encoder())
        .with_predicate(FSEPredicate::range(
            FSEPredicateField::name("score"),
            FSEValue::Float(10.0),
            FSEValue::Float(20.0),
        ))
        .with_predicate(FSEPredicate::equals(
            FSEPredicateField::name("class"),
            FSEValue::Category("alpha".to_string()),
        ))
        .build()
        .expect("valid predicates should produce a plan")
}

fn score_range_plan(
    schema: &FSESchema,
    mapping: &FSESchemaDimensionMapping,
    min: f64,
    max: f64,
) -> TypedQueryPlan {
    TypedQueryPlanBuilder::new(schema, mapping)
        .with_predicate(FSEPredicate::range(
            FSEPredicateField::name("score"),
            FSEValue::Float(min),
            FSEValue::Float(max),
        ))
        .build()
        .expect("valid predicate should produce a plan")
}

fn class_equality_plan(
    schema: &FSESchema,
    mapping: &FSESchemaDimensionMapping,
    class: &str,
) -> TypedQueryPlan {
    TypedQueryPlanBuilder::new(schema, mapping)
        .with_categorical_encoder(2, class_encoder())
        .with_predicate(FSEPredicate::equals(
            FSEPredicateField::name("class"),
            FSEValue::Category(class.to_string()),
        ))
        .build()
        .expect("valid predicate should produce a plan")
}

fn entity_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new("entity_id", FSEFieldType::Integer, false),
        FSEField::new("score", FSEFieldType::Float, false),
        FSEField::new("class", FSEFieldType::Category, false),
        FSEField::new("observed_at", FSEFieldType::TimestampMillis, false),
    ])
}

fn entity_mapping(schema: &FSESchema) -> FSESchemaDimensionMapping {
    FSESchemaDimensionMapping::new(
        schema,
        vec![
            FSEDimensionMapping::new(0, 0),
            FSEDimensionMapping::new(1, 1),
            FSEDimensionMapping::new(2, 2),
            FSEDimensionMapping::new(3, 3),
        ],
    )
}

fn entity_encoder(schema: &FSESchema) -> ComposedRecordEncoder {
    ComposedRecordEncoder::new(
        schema,
        vec![
            Box::new(IntegerEncoder),
            Box::new(FloatEncoder),
            Box::new(class_encoder()),
            Box::new(TimestampMillisEncoder),
        ],
    )
}

fn entity_batch(schema: &FSESchema) -> FSERecordBatch {
    FSERecordBatch::new(
        schema.clone(),
        vec![
            RowId::new(100),
            RowId::new(101),
            RowId::new(102),
            RowId::new(103),
        ],
        vec![
            entity_record(schema, 1, 12.5, "alpha", 1_000),
            entity_record(schema, 2, 12.5, "beta", 1_100),
            entity_record(schema, 3, 25.0, "alpha", 1_200),
            entity_record(schema, 4, 18.0, "alpha", 1_300),
        ],
    )
}

fn broad_class_batch(schema: &FSESchema) -> FSERecordBatch {
    FSERecordBatch::new(
        schema.clone(),
        vec![
            RowId::new(100),
            RowId::new(101),
            RowId::new(102),
            RowId::new(103),
        ],
        vec![
            entity_record(schema, 1, 12.5, "alpha", 1_000),
            entity_record(schema, 2, 13.0, "alpha", 1_100),
            entity_record(schema, 3, 25.0, "alpha", 1_200),
            entity_record(schema, 4, 18.0, "beta", 1_300),
        ],
    )
}

fn appended_entity_batch(schema: &FSESchema) -> FSERecordBatch {
    FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(104), RowId::new(105)],
        vec![
            entity_record(schema, 5, 16.0, "alpha", 1_400),
            entity_record(schema, 6, 80.0, "beta", 1_500),
        ],
    )
}

fn entity_record(
    schema: &FSESchema,
    entity_id: i64,
    score: f64,
    class: &str,
    observed_at: i64,
) -> FSERecord {
    FSERecord::new(
        vec![
            FSEValue::Integer(entity_id),
            FSEValue::Float(score),
            FSEValue::Category(class.to_string()),
            FSEValue::TimestampMillis(observed_at),
        ],
        schema,
    )
}

fn class_encoder() -> CategoricalDictionaryEncoder {
    CategoricalDictionaryEncoder::new(vec!["alpha".to_string(), "beta".to_string()])
}

fn result_row_ids(rows: &[TypedQueryResultRow]) -> Vec<RowId> {
    rows.iter().map(|row| row.row_id()).collect()
}

fn builder() -> FSEBuilder {
    FSEBuilder::new(BuildConfig::new(2, 8))
}
