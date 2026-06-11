//! Typed benchmark workload setup.

use super::super::context::BenchmarkApplicationContext;
use crate::benchmark::workloads::QueryWorkloadCase;
use crate::build::FSEBuilder;
use crate::data::{
    FSEDimensionMapping, FSEField, FSEFieldType, FSERecord, FSERecordBatch, FSESchema,
    FSESchemaDimensionMapping, FSEValue, RowId,
};
use crate::encoding::{ComposedRecordEncoder, FloatEncoder};
use crate::query::{FSEPredicate, FSEPredicateField, TypedQueryIndex, TypedQueryPlan};

const X_FIELD_NAME: &str = "x";
const Y_FIELD_NAME: &str = "y";
const APPENDED_RECORD_LIMIT: usize = 64;

pub(super) struct TypedBenchmarkContext {
    schema: FSESchema,
    mapping: FSESchemaDimensionMapping,
    query_index: TypedQueryIndex,
}

impl TypedBenchmarkContext {
    pub(super) fn from_benchmark_context(context: &BenchmarkApplicationContext) -> Self {
        let schema = typed_benchmark_schema();
        let mapping = typed_benchmark_mapping(&schema);
        let batch = typed_benchmark_batch(&schema, context);
        let encoder = typed_benchmark_encoder(&schema);
        let builder = FSEBuilder::new(context.suite_config.build_config());
        let query_index = TypedQueryIndex::try_build(batch, &encoder, &builder)
            .expect("typed benchmark records should build a typed query index");

        Self {
            schema,
            mapping,
            query_index,
        }
    }

    pub(super) fn query_index(&self) -> &TypedQueryIndex {
        &self.query_index
    }

    pub(super) fn encoder(&self) -> ComposedRecordEncoder {
        typed_benchmark_encoder(&self.schema)
    }

    pub(super) fn append_batch_from_benchmark_context(
        &self,
        context: &BenchmarkApplicationContext,
    ) -> FSERecordBatch {
        let base_record_count = self.query_index.batch().len();
        let appended_record_count = context.points.len().min(APPENDED_RECORD_LIMIT);
        let row_ids: Vec<RowId> = (0..appended_record_count)
            .map(|index| RowId::new((base_record_count + index) as u64))
            .collect();
        let records: Vec<FSERecord> = context
            .points
            .iter()
            .take(appended_record_count)
            .map(|point| {
                FSERecord::new(
                    vec![
                        FSEValue::Float(point.values[0] as f64),
                        FSEValue::Float(point.values[1] as f64),
                    ],
                    &self.schema,
                )
            })
            .collect();

        FSERecordBatch::new(self.schema.clone(), row_ids, records)
    }
}

pub(super) fn typed_x_range_plan(
    typed_context: &TypedBenchmarkContext,
    workload: &QueryWorkloadCase,
) -> TypedQueryPlan {
    let predicate = FSEPredicate::range(
        FSEPredicateField::name(X_FIELD_NAME),
        FSEValue::Float(workload.query.min[0] as f64),
        FSEValue::Float(workload.query.max[0] as f64),
    );

    TypedQueryPlan::numeric(&predicate, &typed_context.schema, &typed_context.mapping)
        .expect("typed x-range predicate should produce a plan")
}

fn typed_benchmark_schema() -> FSESchema {
    FSESchema::new(vec![
        FSEField::new(X_FIELD_NAME, FSEFieldType::Float, false),
        FSEField::new(Y_FIELD_NAME, FSEFieldType::Float, false),
    ])
}

fn typed_benchmark_mapping(schema: &FSESchema) -> FSESchemaDimensionMapping {
    FSESchemaDimensionMapping::new(
        schema,
        vec![
            FSEDimensionMapping::new(0, 0),
            FSEDimensionMapping::new(1, 1),
        ],
    )
}

fn typed_benchmark_encoder(schema: &FSESchema) -> ComposedRecordEncoder {
    ComposedRecordEncoder::new(schema, vec![Box::new(FloatEncoder), Box::new(FloatEncoder)])
}

fn typed_benchmark_batch(
    schema: &FSESchema,
    context: &BenchmarkApplicationContext,
) -> FSERecordBatch {
    let row_ids: Vec<RowId> = (0..context.points.len())
        .map(|index| RowId::new(index as u64))
        .collect();
    let records: Vec<FSERecord> = context
        .points
        .iter()
        .map(|point| {
            FSERecord::new(
                vec![
                    FSEValue::Float(point.values[0] as f64),
                    FSEValue::Float(point.values[1] as f64),
                ],
                schema,
            )
        })
        .collect();

    FSERecordBatch::new(schema.clone(), row_ids, records)
}
