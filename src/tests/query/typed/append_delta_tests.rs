use crate::build::{BuildConfig, FSEBuilder};
use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSERecordBatchError,
    FSESchema, FSESchemaDimensionMapping, FSEValue, RowId,
};
use crate::encoding::{
    CategoricalDictionaryEncoder, ComposedRecordEncoder, FloatEncoder, IntegerEncoder,
    TimestampMillisEncoder,
};
use crate::query::{
    FSEPredicate, FSEPredicateField, TypedAppendDeltaQueryView, TypedQueryIndex,
    TypedQueryOutputContract, TypedQueryPlanBuilder, TypedRowTombstoneSet,
    TypedTombstonedAppendDeltaQueryView,
};

#[test]
fn typed_append_delta_view_queries_base_and_appended_row_ids() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let base_batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let builder = builder();
    let base_index = TypedQueryIndex::try_build(base_batch.clone(), &encoder, &builder)
        .expect("valid base index should build");
    let base_cardinality = base_index.index().index().root_node().cardinality;
    let plan = score_and_class_plan(&schema, &mapping);

    let view = TypedAppendDeltaQueryView::try_new(&base_index, &appended)
        .expect("valid append delta should build a query view");
    let matches = view
        .query_row_ids(&plan)
        .expect("append delta query should execute");

    assert_eq!(
        matches,
        vec![RowId::new(100), RowId::new(103), RowId::new(104)]
    );
    assert_eq!(view.base().batch(), &base_batch);
    assert_eq!(view.appended(), &appended);
    assert_eq!(
        base_index.index().index().root_node().cardinality,
        base_cardinality
    );
}

#[test]
fn typed_append_delta_view_returns_rows_from_appended_batch() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let base_batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let base_index = TypedQueryIndex::try_build(base_batch, &encoder, &builder())
        .expect("valid base index should build");
    let view = TypedAppendDeltaQueryView::try_new(&base_index, &appended)
        .expect("valid append delta should build a query view");
    let plan = score_range_plan(&schema, &mapping, 15.0, 17.0);

    let rows = view
        .query_rows(&plan)
        .expect("append delta row query should execute");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].row_id(), RowId::new(104));
    assert_eq!(
        rows[0].record(),
        appended.record_for_row_id(RowId::new(104)).unwrap()
    );
}

#[test]
fn typed_append_delta_view_counts_and_reports_existence_across_batches() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let base_batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let base_index = TypedQueryIndex::try_build(base_batch, &encoder, &builder())
        .expect("valid base index should build");
    let view = TypedAppendDeltaQueryView::try_new(&base_index, &appended)
        .expect("valid append delta should build a query view");
    let matching_plan = score_and_class_plan(&schema, &mapping);
    let missing_plan = score_range_plan(&schema, &mapping, 90.0, 100.0);

    assert_eq!(
        view.count_matches(&matching_plan)
            .expect("append delta count should execute"),
        3
    );
    assert!(
        view.has_match(&matching_plan)
            .expect("append delta existence should execute")
    );
    assert!(
        !view
            .has_match(&missing_plan)
            .expect("append delta existence should execute")
    );
}

#[test]
fn typed_append_delta_view_reports_stats_across_base_and_appended_records() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let base_batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let base_index = TypedQueryIndex::try_build(base_batch, &encoder, &builder())
        .expect("valid base index should build");
    let view = TypedAppendDeltaQueryView::try_new(&base_index, &appended)
        .expect("valid append delta should build a query view");
    let plan = score_and_class_plan(&schema, &mapping);
    let base_report = base_index
        .query_row_ids_with_stats(&plan)
        .expect("base query stats should execute");

    let row_report = view
        .query_row_ids_with_stats(&plan)
        .expect("append delta row-id stats should execute");
    let row_result_report = view
        .query_rows_with_stats(&plan)
        .expect("append delta row stats should execute");
    let count_report = view
        .count_matches_with_stats(&plan)
        .expect("append delta count stats should execute");

    assert_eq!(
        row_report.row_ids,
        vec![RowId::new(100), RowId::new(103), RowId::new(104)]
    );
    assert_eq!(
        row_result_report
            .rows
            .iter()
            .map(|row| row.row_id())
            .collect::<Vec<_>>(),
        row_report.row_ids
    );
    assert_eq!(count_report.matched_records, 3);
    assert_eq!(row_report.stats.total_records, 6);
    assert_eq!(
        row_report.stats.reconstructed_records,
        base_report.stats.reconstructed_records + appended.len()
    );
    assert_eq!(row_report.stats.matched_records, 3);
    assert_eq!(row_result_report.stats, row_report.stats);
    assert_eq!(count_report.stats, row_report.stats);
}

#[test]
fn typed_append_delta_view_reports_stats_with_tombstones() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let base_batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let base_index = TypedQueryIndex::try_build(base_batch, &encoder, &builder())
        .expect("valid base index should build");
    let view = TypedAppendDeltaQueryView::try_new(&base_index, &appended)
        .expect("valid append delta should build a query view");
    let tombstones = TypedRowTombstoneSet::from_row_ids([RowId::new(103), RowId::new(104)]);
    let plan = score_and_class_plan(&schema, &mapping);
    let base_report = base_index
        .query_row_ids_with_stats_excluding_tombstones(&plan, &tombstones)
        .expect("base query stats should execute");

    let row_report = view
        .query_row_ids_with_stats_excluding_tombstones(&plan, &tombstones)
        .expect("append delta row-id stats should execute");
    let row_result_report = view
        .query_rows_with_stats_excluding_tombstones(&plan, &tombstones)
        .expect("append delta row stats should execute");
    let count_report = view
        .count_matches_with_stats_excluding_tombstones(&plan, &tombstones)
        .expect("append delta count stats should execute");

    assert_eq!(row_report.row_ids, vec![RowId::new(100)]);
    assert_eq!(
        row_result_report
            .rows
            .iter()
            .map(|row| row.row_id())
            .collect::<Vec<_>>(),
        row_report.row_ids
    );
    assert_eq!(count_report.matched_records, 1);
    assert_eq!(row_report.stats.total_records, 6);
    assert_eq!(
        row_report.stats.reconstructed_records,
        base_report.stats.reconstructed_records + appended.len()
    );
    assert_eq!(row_report.stats.matched_records, 1);
    assert_eq!(row_result_report.stats, row_report.stats);
    assert_eq!(count_report.stats, row_report.stats);
}

#[test]
fn typed_tombstoned_append_delta_view_routes_result_contracts() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let base_batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let base_index = TypedQueryIndex::try_build(base_batch, &encoder, &builder())
        .expect("valid base index should build");
    let view = TypedAppendDeltaQueryView::try_new(&base_index, &appended)
        .expect("valid append delta should build a query view");
    let tombstones = TypedRowTombstoneSet::from_row_ids([RowId::new(103), RowId::new(104)]);
    let tombstoned = TypedTombstonedAppendDeltaQueryView::from_view(view, &tombstones);
    let plan = score_and_class_plan(&schema, &mapping);
    let row_report = tombstoned
        .query_row_ids_with_stats(&plan)
        .expect("tombstoned append delta row-id stats should execute");
    let planned_row_report = tombstoned
        .query_row_ids_with_planning(&plan)
        .expect("tombstoned append delta planned row-id query should execute");
    let row_result_report = tombstoned
        .query_rows_with_stats(&plan)
        .expect("tombstoned append delta row stats should execute");
    let planned_row_result_report = tombstoned
        .query_rows_with_planning(&plan)
        .expect("tombstoned append delta planned row query should execute");
    let count_report = tombstoned
        .count_matches_with_stats(&plan)
        .expect("tombstoned append delta count stats should execute");
    let planned_count_report = tombstoned
        .count_matches_with_planning(&plan)
        .expect("tombstoned append delta planned count should execute");
    let existence_report = tombstoned
        .has_match_with_stats(&plan)
        .expect("tombstoned append delta existence stats should execute");
    let planned_existence_report = tombstoned
        .has_match_with_planning(&plan)
        .expect("tombstoned append delta planned existence should execute");
    let mut visited_row_ids = Vec::new();
    tombstoned
        .visit_row_ids(&plan, |row_id| {
            visited_row_ids.push(row_id);
        })
        .expect("tombstoned append delta row-id visitor should execute");
    let mut planned_visited_row_ids = Vec::new();
    let planned_row_id_visit_report = tombstoned
        .visit_row_ids_with_planning(&plan, |row_id| {
            planned_visited_row_ids.push(row_id);
        })
        .expect("tombstoned append delta planned row-id visitor should execute");
    let mut visited_rows = Vec::new();
    tombstoned
        .visit_rows(&plan, |row_id, record| {
            visited_rows.push((row_id, record.clone()));
        })
        .expect("tombstoned append delta row visitor should execute");
    let mut planned_visited_rows = Vec::new();
    let planned_row_visit_report = tombstoned
        .visit_rows_with_planning(&plan, |row_id, record| {
            planned_visited_rows.push((row_id, record.clone()));
        })
        .expect("tombstoned append delta planned row visitor should execute");

    assert_eq!(tombstoned.tombstones(), &tombstones);
    assert_eq!(
        tombstoned
            .query_row_ids(&plan)
            .expect("tombstoned append delta query should execute"),
        vec![RowId::new(100)]
    );
    assert_eq!(row_report.row_ids, vec![RowId::new(100)]);
    assert_eq!(planned_row_report.row_ids, row_report.row_ids);
    assert_eq!(
        planned_row_report.diagnostics.output_contract,
        TypedQueryOutputContract::RowIds
    );
    assert_eq!(
        tombstoned
            .query_rows(&plan)
            .expect("tombstoned append delta row query should execute")[0]
            .row_id(),
        RowId::new(100)
    );
    assert_eq!(row_result_report.rows.len(), 1);
    assert_eq!(planned_row_result_report.rows.len(), 1);
    assert_eq!(
        planned_row_result_report.diagnostics.output_contract,
        TypedQueryOutputContract::Rows
    );
    assert_eq!(
        tombstoned
            .count_matches(&plan)
            .expect("tombstoned append delta count should execute"),
        1
    );
    assert_eq!(count_report.matched_records, 1);
    assert_eq!(planned_count_report.matched_records, 1);
    assert_eq!(
        planned_count_report.diagnostics.output_contract,
        TypedQueryOutputContract::Count
    );
    assert!(
        tombstoned
            .has_match(&plan)
            .expect("tombstoned append delta existence should execute")
    );
    assert!(existence_report.has_match);
    assert!(planned_existence_report.has_match);
    assert_eq!(
        planned_existence_report.diagnostics.output_contract,
        TypedQueryOutputContract::Existence
    );
    assert_eq!(visited_row_ids, row_report.row_ids);
    assert_eq!(planned_visited_row_ids, row_report.row_ids);
    assert_eq!(planned_row_id_visit_report.visited_records, 1);
    assert_eq!(
        planned_row_id_visit_report.diagnostics.output_contract,
        TypedQueryOutputContract::RowIdVisitor
    );
    assert_eq!(visited_rows.len(), 1);
    assert_eq!(planned_visited_rows.len(), 1);
    assert_eq!(planned_row_visit_report.visited_records, 1);
    assert_eq!(
        planned_row_visit_report.diagnostics.output_contract,
        TypedQueryOutputContract::RowVisitor
    );
}

#[test]
fn typed_append_delta_view_reports_existence_stats_across_batches() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let base_batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let base_index = TypedQueryIndex::try_build(base_batch, &encoder, &builder())
        .expect("valid base index should build");
    let view = TypedAppendDeltaQueryView::try_new(&base_index, &appended)
        .expect("valid append delta should build a query view");
    let appended_only_plan = score_range_plan(&schema, &mapping, 16.0, 16.0);
    let tombstones = TypedRowTombstoneSet::from_row_ids([RowId::new(104)]);

    let match_report = view
        .has_match_with_stats(&appended_only_plan)
        .expect("append delta existence stats should execute");
    let tombstoned_report = view
        .has_match_with_stats_excluding_tombstones(&appended_only_plan, &tombstones)
        .expect("append delta tombstoned existence stats should execute");

    assert!(match_report.has_match);
    assert_eq!(match_report.stats.total_records, 6);
    assert_eq!(match_report.stats.matched_records, 1);
    assert!(!tombstoned_report.has_match);
    assert_eq!(tombstoned_report.stats.total_records, 6);
    assert_eq!(tombstoned_report.stats.matched_records, 0);
    assert!(tombstoned_report.inspected_records >= match_report.inspected_records);
}

#[test]
fn typed_append_delta_view_visits_base_and_appended_matches() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let base_batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let base_index = TypedQueryIndex::try_build(base_batch, &encoder, &builder())
        .expect("valid base index should build");
    let view = TypedAppendDeltaQueryView::try_new(&base_index, &appended)
        .expect("valid append delta should build a query view");
    let plan = score_and_class_plan(&schema, &mapping);
    let mut row_ids = Vec::new();
    let mut rows = Vec::new();

    view.visit_row_ids(&plan, |row_id| {
        row_ids.push(row_id);
    })
    .expect("append delta row-id visitor should execute");
    view.visit_rows(&plan, |row_id, record| {
        rows.push((row_id, record.clone()));
    })
    .expect("append delta row visitor should execute");

    assert_eq!(
        row_ids,
        vec![RowId::new(100), RowId::new(103), RowId::new(104)]
    );
    assert_eq!(
        rows.iter()
            .map(|(row_id, _record)| *row_id)
            .collect::<Vec<_>>(),
        row_ids
    );
}

#[test]
fn typed_append_delta_view_excludes_tombstones_from_row_id_queries() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let base_batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let base_index = TypedQueryIndex::try_build(base_batch, &encoder, &builder())
        .expect("valid base index should build");
    let view = TypedAppendDeltaQueryView::try_new(&base_index, &appended)
        .expect("valid append delta should build a query view");
    let tombstones = TypedRowTombstoneSet::from_row_ids([RowId::new(103), RowId::new(104)]);
    let plan = score_and_class_plan(&schema, &mapping);

    let unfiltered = view
        .query_row_ids(&plan)
        .expect("append delta query should execute");
    let filtered = view
        .query_row_ids_excluding_tombstones(&plan, &tombstones)
        .expect("append delta query should execute");

    assert_eq!(
        unfiltered,
        vec![RowId::new(100), RowId::new(103), RowId::new(104)]
    );
    assert_eq!(filtered, vec![RowId::new(100)]);
}

#[test]
fn typed_append_delta_view_excludes_tombstones_from_row_queries() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let base_batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let base_index = TypedQueryIndex::try_build(base_batch, &encoder, &builder())
        .expect("valid base index should build");
    let view = TypedAppendDeltaQueryView::try_new(&base_index, &appended)
        .expect("valid append delta should build a query view");
    let tombstones = TypedRowTombstoneSet::from_row_ids([RowId::new(103), RowId::new(104)]);
    let plan = score_and_class_plan(&schema, &mapping);

    let rows = view
        .query_rows_excluding_tombstones(&plan, &tombstones)
        .expect("append delta row query should execute");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].row_id(), RowId::new(100));
}

#[test]
fn typed_append_delta_view_excludes_tombstones_from_count_and_existence() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let base_batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let base_index = TypedQueryIndex::try_build(base_batch, &encoder, &builder())
        .expect("valid base index should build");
    let view = TypedAppendDeltaQueryView::try_new(&base_index, &appended)
        .expect("valid append delta should build a query view");
    let tombstones = TypedRowTombstoneSet::from_row_ids([RowId::new(103), RowId::new(104)]);
    let matching_plan = score_and_class_plan(&schema, &mapping);
    let tombstoned_base_plan = score_range_plan(&schema, &mapping, 18.0, 18.0);
    let tombstoned_append_plan = score_range_plan(&schema, &mapping, 16.0, 16.0);

    assert_eq!(
        view.count_matches_excluding_tombstones(&matching_plan, &tombstones)
            .expect("append delta count should execute"),
        1
    );
    assert!(
        view.has_match_excluding_tombstones(&matching_plan, &tombstones)
            .expect("append delta existence should execute")
    );
    assert!(
        !view
            .has_match_excluding_tombstones(&tombstoned_base_plan, &tombstones)
            .expect("append delta existence should execute")
    );
    assert!(
        !view
            .has_match_excluding_tombstones(&tombstoned_append_plan, &tombstones)
            .expect("append delta existence should execute")
    );
}

#[test]
fn typed_append_delta_view_excludes_tombstones_from_visitors() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let base_batch = entity_batch(&schema);
    let appended = appended_entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let base_index = TypedQueryIndex::try_build(base_batch, &encoder, &builder())
        .expect("valid base index should build");
    let view = TypedAppendDeltaQueryView::try_new(&base_index, &appended)
        .expect("valid append delta should build a query view");
    let tombstones = TypedRowTombstoneSet::from_row_ids([RowId::new(103), RowId::new(104)]);
    let plan = score_and_class_plan(&schema, &mapping);
    let mut row_ids = Vec::new();
    let mut rows = Vec::new();

    view.visit_row_ids_excluding_tombstones(&plan, &tombstones, |row_id| {
        row_ids.push(row_id);
    })
    .expect("append delta row-id visitor should execute");
    view.visit_rows_excluding_tombstones(&plan, &tombstones, |row_id, record| {
        rows.push((row_id, record.clone()));
    })
    .expect("append delta row visitor should execute");

    assert_eq!(row_ids, vec![RowId::new(100)]);
    assert_eq!(
        rows.iter()
            .map(|(row_id, _record)| *row_id)
            .collect::<Vec<_>>(),
        row_ids
    );
}

#[test]
fn typed_append_delta_view_validates_append_batch_boundaries() {
    let schema = entity_schema();
    let encoder = entity_encoder(&schema);
    let base_index = TypedQueryIndex::try_build(entity_batch(&schema), &encoder, &builder())
        .expect("valid base index should build");
    let duplicate = FSERecordBatch::new(
        schema.clone(),
        vec![RowId::new(100)],
        vec![entity_record(&schema, 5, 16.0, "alpha", 1_400)],
    );
    let empty = FSERecordBatch::new(schema.clone(), Vec::new(), Vec::new());
    let mismatched_schema = FSESchema::new(vec![FSEField::new(
        "entity_id",
        FSEFieldType::Integer,
        false,
    )]);
    let mismatched = FSERecordBatch::new(
        mismatched_schema.clone(),
        vec![RowId::new(104)],
        vec![FSERecord::new(
            vec![FSEValue::Integer(5)],
            &mismatched_schema,
        )],
    );

    assert_eq!(
        TypedAppendDeltaQueryView::try_new(&base_index, &duplicate)
            .expect_err("duplicate row id should be rejected"),
        FSERecordBatchError::DuplicateRowId {
            row_id: RowId::new(100)
        }
    );
    assert_eq!(
        TypedAppendDeltaQueryView::try_new(&base_index, &empty)
            .expect_err("empty append batch should be rejected"),
        FSERecordBatchError::EmptyAppendBatch
    );
    assert_eq!(
        TypedAppendDeltaQueryView::try_new(&base_index, &mismatched)
            .expect_err("schema mismatch should be rejected"),
        FSERecordBatchError::SchemaMismatch
    );
}

fn score_and_class_plan(
    schema: &FSESchema,
    mapping: &FSESchemaDimensionMapping,
) -> crate::query::TypedQueryPlan {
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
) -> crate::query::TypedQueryPlan {
    TypedQueryPlanBuilder::new(schema, mapping)
        .with_predicate(FSEPredicate::range(
            FSEPredicateField::name("score"),
            FSEValue::Float(min),
            FSEValue::Float(max),
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

fn builder() -> FSEBuilder {
    FSEBuilder::new(BuildConfig::new(2, 8))
}
