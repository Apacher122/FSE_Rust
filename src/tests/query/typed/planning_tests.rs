use crate::build::{BuildConfig, FSEBuilder};
use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSESchema,
    FSESchemaDimensionMapping, FSEValue, RowId,
};
use crate::encoding::{
    CategoricalDictionaryEncoder, ComposedRecordEncoder, FSEFieldEncoder, FloatEncoder,
    IntegerEncoder, TimestampMillisEncoder,
};
use crate::query::{
    FSEPredicate, FSEPredicateField, TypedAppendDeltaQueryView, TypedQueryExecutionStrategy,
    TypedQueryIndex, TypedQueryOutputContract, TypedQueryPlan, TypedQueryPlanBuilder,
    TypedQueryPlanningReason, TypedQuerySelectivityBucket, plan_typed_append_delta_query_execution,
    plan_typed_query_execution,
};

#[test]
fn typed_query_planning_diagnostics_selects_fse_for_selective_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let plan = score_and_class_plan(&schema, &mapping);

    let diagnostics =
        plan_typed_query_execution(&query_index, &plan, TypedQueryOutputContract::RowIds);

    assert_eq!(
        diagnostics.strategy,
        TypedQueryExecutionStrategy::FseTraversal
    );
    assert_eq!(
        diagnostics.reason,
        TypedQueryPlanningReason::SelectiveGeometry
    );
    assert_eq!(
        diagnostics.output_contract,
        TypedQueryOutputContract::RowIds
    );
    assert_eq!(diagnostics.total_records, 4);
    assert_eq!(diagnostics.base_record_count, 4);
    assert_eq!(diagnostics.append_delta_record_count, 0);
    assert_eq!(diagnostics.predicate_count, 2);
    assert_eq!(diagnostics.constrained_dimensions, 2);
    assert!(diagnostics.estimated_candidate_records > 0);
    assert!(diagnostics.estimated_candidate_ratio <= 0.35);
    assert_eq!(
        diagnostics.selectivity_bucket,
        TypedQuerySelectivityBucket::Selective
    );
    assert!(diagnostics.estimated_retained_leaf_count <= diagnostics.leaf_count);
    assert!(!diagnostics.requires_append_delta_scan);
    assert!(diagnostics.uses_geometric_traversal());
    assert!(!diagnostics.uses_flat_scan());
    assert!(!diagnostics.is_broad_query());
    assert!(!diagnostics.risk_flags.has_any());
    assert!(diagnostics.work_estimate.estimated_traversal_node_visits > 0);
    assert_eq!(
        diagnostics.work_estimate,
        diagnostics
            .strategy_costs
            .for_strategy(diagnostics.strategy)
    );
    assert_eq!(
        diagnostics.strategy_costs.fse_traversal,
        diagnostics.work_estimate
    );
    assert_eq!(
        diagnostics
            .strategy_costs
            .flat_scan
            .estimated_predicate_evaluations,
        diagnostics.total_records
    );
    assert_eq!(
        diagnostics
            .strategy_costs
            .flat_scan
            .estimated_flat_scan_records,
        diagnostics.total_records
    );
    let comparison = diagnostics.cost_comparison_against_flat_scan();

    assert_eq!(comparison.selected_strategy, diagnostics.strategy);
    assert_eq!(comparison.selected_work, diagnostics.work_estimate);
    assert_eq!(
        comparison.flat_scan_work,
        diagnostics.strategy_costs.flat_scan
    );
    assert!(comparison.reduces_predicate_evaluations());
    assert!(comparison.reduces_flat_scan_records());
    assert!(comparison.traversal_node_visit_delta > 0);
    assert_eq!(comparison.materialized_record_delta, 0);
    assert_eq!(
        diagnostics.work_estimate.estimated_reconstructed_records,
        diagnostics.estimated_candidate_records
    );
    assert_eq!(
        diagnostics.work_estimate.estimated_predicate_evaluations,
        diagnostics.estimated_candidate_records
    );
    assert_eq!(diagnostics.work_estimate.estimated_materialized_records, 0);
    assert_eq!(diagnostics.work_estimate.estimated_flat_scan_records, 0);
}

#[test]
fn typed_query_planning_diagnostics_selects_flat_scan_for_broad_materialized_plan() {
    let schema = entity_schema();
    let mapping = entity_mapping(&schema);
    let batch = entity_batch(&schema);
    let encoder = entity_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let plan = score_range_plan(&schema, &mapping, 0.0, 100.0);

    let diagnostics =
        plan_typed_query_execution(&query_index, &plan, TypedQueryOutputContract::Rows);

    assert_eq!(diagnostics.strategy, TypedQueryExecutionStrategy::FlatScan);
    assert_eq!(
        diagnostics.reason,
        TypedQueryPlanningReason::BroadMaterializedQuery
    );
    assert_eq!(diagnostics.estimated_candidate_records, 4);
    assert_eq!(diagnostics.estimated_candidate_ratio, 1.0);
    assert_eq!(
        diagnostics.selectivity_bucket,
        TypedQuerySelectivityBucket::Broad
    );
    assert!(diagnostics.output_contract.materializes_records());
    assert!(diagnostics.uses_flat_scan());
    assert!(diagnostics.is_broad_query());
    assert!(diagnostics.risk_flags.has_any());
    assert!(diagnostics.risk_flags.broad_predicate);
    assert!(diagnostics.risk_flags.materialization_pressure);
    assert!(!diagnostics.risk_flags.high_dimensional_low_constraint);
    assert!(!diagnostics.risk_flags.append_delta_scan);
    assert_eq!(
        diagnostics.work_estimate,
        diagnostics
            .strategy_costs
            .for_strategy(diagnostics.strategy)
    );
    assert_eq!(
        diagnostics.strategy_costs.flat_scan,
        diagnostics.work_estimate
    );
    assert_eq!(
        diagnostics
            .strategy_costs
            .fse_traversal
            .estimated_predicate_evaluations,
        diagnostics.estimated_candidate_records
    );
    let comparison = diagnostics.cost_comparison_against_flat_scan();

    assert_eq!(comparison.selected_strategy, diagnostics.strategy);
    assert_eq!(comparison.selected_work, diagnostics.work_estimate);
    assert_eq!(
        comparison.flat_scan_work,
        diagnostics.strategy_costs.flat_scan
    );
    assert_eq!(comparison.predicate_evaluation_delta, 0);
    assert_eq!(comparison.flat_scan_record_delta, 0);
    assert_eq!(comparison.traversal_node_visit_delta, 0);
    assert_eq!(diagnostics.work_estimate.estimated_traversal_node_visits, 0);
    assert_eq!(diagnostics.work_estimate.estimated_reconstructed_records, 0);
    assert_eq!(
        diagnostics.work_estimate.estimated_predicate_evaluations,
        diagnostics.total_records
    );
    assert_eq!(
        diagnostics.work_estimate.estimated_materialized_records,
        diagnostics.estimated_candidate_records
    );
    assert_eq!(
        diagnostics.work_estimate.estimated_flat_scan_records,
        diagnostics.total_records
    );
}

#[test]
fn typed_query_planning_diagnostics_reports_noop_for_unsatisfiable_plan() {
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

    let diagnostics =
        plan_typed_query_execution(&query_index, &plan, TypedQueryOutputContract::Count);

    assert!(plan.is_unsatisfiable());
    assert_eq!(diagnostics.strategy, TypedQueryExecutionStrategy::NoOp);
    assert_eq!(
        diagnostics.reason,
        TypedQueryPlanningReason::UnsatisfiablePlan
    );
    assert_eq!(diagnostics.estimated_candidate_records, 0);
    assert_eq!(diagnostics.estimated_candidate_ratio, 0.0);
    assert_eq!(
        diagnostics.selectivity_bucket,
        TypedQuerySelectivityBucket::Empty
    );
    assert!(!diagnostics.uses_geometric_traversal());
    assert!(!diagnostics.risk_flags.has_any());
    assert!(diagnostics.work_estimate.is_empty());
    assert_eq!(
        diagnostics.work_estimate,
        diagnostics
            .strategy_costs
            .for_strategy(diagnostics.strategy)
    );
    assert!(diagnostics.strategy_costs.no_op.is_empty());
    let comparison = diagnostics.cost_comparison_against_flat_scan();

    assert_eq!(comparison.selected_strategy, diagnostics.strategy);
    assert_eq!(comparison.selected_work, diagnostics.work_estimate);
    assert_eq!(
        comparison.flat_scan_work,
        diagnostics.strategy_costs.flat_scan
    );
    assert!(comparison.reduces_predicate_evaluations());
    assert!(comparison.reduces_flat_scan_records());
    assert_eq!(comparison.traversal_node_visit_delta, 0);
}

#[test]
fn typed_query_planning_diagnostics_selects_hybrid_for_append_delta_view() {
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

    let diagnostics =
        plan_typed_append_delta_query_execution(&view, &plan, TypedQueryOutputContract::RowIds);

    assert_eq!(diagnostics.strategy, TypedQueryExecutionStrategy::Hybrid);
    assert_eq!(
        diagnostics.reason,
        TypedQueryPlanningReason::AppendDeltaScan
    );
    assert_eq!(diagnostics.total_records, 6);
    assert_eq!(diagnostics.base_record_count, 4);
    assert_eq!(diagnostics.append_delta_record_count, 2);
    assert!(diagnostics.estimated_candidate_records >= 2);
    assert_eq!(
        diagnostics.selectivity_bucket,
        TypedQuerySelectivityBucket::Moderate
    );
    assert!(diagnostics.requires_append_delta_scan);
    assert!(diagnostics.uses_geometric_traversal());
    assert!(diagnostics.risk_flags.has_any());
    assert!(!diagnostics.risk_flags.broad_predicate);
    assert!(!diagnostics.risk_flags.materialization_pressure);
    assert!(!diagnostics.risk_flags.high_dimensional_low_constraint);
    assert!(diagnostics.risk_flags.append_delta_scan);
    assert_eq!(
        diagnostics.work_estimate,
        diagnostics
            .strategy_costs
            .for_strategy(diagnostics.strategy)
    );
    assert_eq!(diagnostics.strategy_costs.hybrid, diagnostics.work_estimate);
    assert_eq!(
        diagnostics
            .strategy_costs
            .flat_scan
            .estimated_predicate_evaluations,
        diagnostics.total_records
    );
    let comparison = diagnostics.cost_comparison_against_flat_scan();

    assert_eq!(comparison.selected_strategy, diagnostics.strategy);
    assert_eq!(comparison.selected_work, diagnostics.work_estimate);
    assert_eq!(
        comparison.flat_scan_work,
        diagnostics.strategy_costs.flat_scan
    );
    assert!(comparison.reduces_predicate_evaluations());
    assert!(comparison.reduces_flat_scan_records());
    assert!(comparison.traversal_node_visit_delta > 0);
    assert!(diagnostics.work_estimate.estimated_traversal_node_visits > 0);
    assert_eq!(
        diagnostics.work_estimate.estimated_flat_scan_records,
        diagnostics.append_delta_record_count
    );
    assert!(
        diagnostics.work_estimate.estimated_predicate_evaluations
            >= diagnostics.append_delta_record_count
    );
    assert_eq!(diagnostics.work_estimate.estimated_materialized_records, 0);
}

#[test]
fn typed_query_planning_diagnostics_flags_high_dimensional_low_constraint_query() {
    let schema = high_dimensional_schema();
    let mapping = high_dimensional_mapping(&schema);
    let batch = high_dimensional_batch(&schema);
    let encoder = high_dimensional_encoder(&schema);
    let query_index =
        TypedQueryIndex::try_build(batch, &encoder, &builder()).expect("valid input should build");
    let plan = high_dimensional_low_constraint_plan(&schema, &mapping);

    let diagnostics =
        plan_typed_query_execution(&query_index, &plan, TypedQueryOutputContract::RowIds);

    assert_eq!(diagnostics.strategy, TypedQueryExecutionStrategy::FlatScan);
    assert_eq!(
        diagnostics.reason,
        TypedQueryPlanningReason::HighDimensionalLowConstraintQuery
    );
    assert_eq!(
        diagnostics.selectivity_bucket,
        TypedQuerySelectivityBucket::Broad
    );
    assert_eq!(diagnostics.dimensions, 8);
    assert_eq!(diagnostics.constrained_dimensions, 1);
    assert!(diagnostics.is_broad_query());
    assert!(diagnostics.risk_flags.has_any());
    assert!(diagnostics.risk_flags.broad_predicate);
    assert!(!diagnostics.risk_flags.materialization_pressure);
    assert!(diagnostics.risk_flags.high_dimensional_low_constraint);
    assert!(!diagnostics.risk_flags.append_delta_scan);
    assert_eq!(
        diagnostics.work_estimate,
        diagnostics
            .strategy_costs
            .for_strategy(diagnostics.strategy)
    );
    assert_eq!(
        diagnostics.strategy_costs.flat_scan,
        diagnostics.work_estimate
    );
    let comparison = diagnostics.cost_comparison_against_flat_scan();

    assert_eq!(comparison.selected_strategy, diagnostics.strategy);
    assert_eq!(comparison.selected_work, diagnostics.work_estimate);
    assert_eq!(
        comparison.flat_scan_work,
        diagnostics.strategy_costs.flat_scan
    );
    assert_eq!(comparison.predicate_evaluation_delta, 0);
    assert_eq!(comparison.flat_scan_record_delta, 0);
    assert_eq!(comparison.traversal_node_visit_delta, 0);
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

fn high_dimensional_schema() -> FSESchema {
    FSESchema::new(
        (0..8)
            .map(|dimension| {
                FSEField::new(format!("metric_{dimension}"), FSEFieldType::Float, false)
            })
            .collect(),
    )
}

fn high_dimensional_mapping(schema: &FSESchema) -> FSESchemaDimensionMapping {
    FSESchemaDimensionMapping::new(
        schema,
        (0..8)
            .map(|dimension| FSEDimensionMapping::new(dimension, dimension))
            .collect(),
    )
}

fn high_dimensional_encoder(schema: &FSESchema) -> ComposedRecordEncoder {
    let encoders = (0..8)
        .map(|_| Box::new(FloatEncoder) as Box<dyn FSEFieldEncoder>)
        .collect();

    ComposedRecordEncoder::new(schema, encoders)
}

fn high_dimensional_batch(schema: &FSESchema) -> FSERecordBatch {
    FSERecordBatch::new(
        schema.clone(),
        vec![
            RowId::new(200),
            RowId::new(201),
            RowId::new(202),
            RowId::new(203),
        ],
        vec![
            high_dimensional_record(schema, 0.0),
            high_dimensional_record(schema, 4.0),
            high_dimensional_record(schema, 8.0),
            high_dimensional_record(schema, 10.0),
        ],
    )
}

fn high_dimensional_record(schema: &FSESchema, first_metric: f64) -> FSERecord {
    FSERecord::new(
        (0..8)
            .map(|dimension| {
                if dimension == 0 {
                    FSEValue::Float(first_metric)
                } else {
                    FSEValue::Float(dimension as f64)
                }
            })
            .collect(),
        schema,
    )
}

fn high_dimensional_low_constraint_plan(
    schema: &FSESchema,
    mapping: &FSESchemaDimensionMapping,
) -> TypedQueryPlan {
    TypedQueryPlanBuilder::new(schema, mapping)
        .with_predicate(FSEPredicate::range(
            FSEPredicateField::name("metric_0"),
            FSEValue::Float(0.0),
            FSEValue::Float(8.0),
        ))
        .build()
        .expect("valid high-dimensional predicate should produce a plan")
}
