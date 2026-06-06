//! Typed indexed query diagnostics.

use super::super::context::BenchmarkApplicationContext;
use super::super::renderer::BenchmarkApplicationRenderer;
use super::formatting::{format_percent_ratio, format_speedup_ratio};
use super::target::{
    append_debug_duration_line, append_debug_line, append_target_workload_debug_section,
};
use crate::benchmark::reports::output::format_duration_ascii;
use crate::benchmark::reports::{
    TypedQueryComparisonReport, compare_typed_query_execution_repeated,
};
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

struct TypedBenchmarkContext {
    schema: FSESchema,
    mapping: FSESchemaDimensionMapping,
    query_index: TypedQueryIndex,
}

impl BenchmarkApplicationRenderer {
    pub(crate) fn append_target_workload_typed_indexed_comparison_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        let typed_context = TypedBenchmarkContext::from_benchmark_context(context);

        append_target_workload_debug_section(
            output,
            context,
            "Target workload typed indexed comparison",
            |output, context, workload| {
                let report = typed_comparison_report(context, &typed_context, workload);

                append_target_typed_comparison_report(output, &report);
            },
        );
    }

    pub(crate) fn append_workload_typed_indexed_comparison_summary_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        let typed_context = TypedBenchmarkContext::from_benchmark_context(context);

        output.push_str("Workload typed indexed comparison summary\n");
        output.push_str("-----------------------------------------\n");
        output.push_str(
            "workload | matched | typed scan | indexed typed | timing ratio | candidate ratio | record avoidance | agreement\n",
        );

        for workload in &context.workloads {
            let report = typed_comparison_report(context, &typed_context, workload);

            output.push_str(&format!(
                "{} | {} | {} | {} | {} | {} | {} | pass\n",
                workload.name,
                report.indexed_matched_records,
                format_duration_ascii(report.repeated_timing.baseline.average_elapsed),
                format_duration_ascii(report.repeated_timing.fse.average_elapsed),
                format_speedup_ratio(report.average_timing_ratio),
                format_scalar_percent(report.candidate_ratio),
                format_scalar_percent(report.record_evaluation_avoidance_ratio),
            ));
        }

        output.push('\n');
    }
}

fn append_target_typed_comparison_report(output: &mut String, report: &TypedQueryComparisonReport) {
    append_debug_line(
        output,
        "baseline matched records",
        report.baseline_matched_records,
    );
    append_debug_line(
        output,
        "indexed matched records",
        report.indexed_matched_records,
    );
    append_debug_duration_line(
        output,
        "typed scan average elapsed",
        report.repeated_timing.baseline.average_elapsed,
    );
    append_debug_duration_line(
        output,
        "indexed typed average elapsed",
        report.repeated_timing.fse.average_elapsed,
    );
    append_debug_line(
        output,
        "indexed typed speedup",
        format_speedup_ratio(report.average_timing_ratio),
    );
    append_debug_line(
        output,
        "candidate ratio",
        format_scalar_percent(report.candidate_ratio),
    );
    append_debug_line(
        output,
        "retained leaf ratio",
        format_scalar_percent(report.retained_leaf_ratio),
    );
    append_debug_line(
        output,
        "avoided record evaluations",
        report.avoided_record_evaluations,
    );
    append_debug_line(
        output,
        "record evaluation avoidance ratio",
        format_scalar_percent(report.record_evaluation_avoidance_ratio),
    );
    append_debug_line(output, "typed comparison agreement", "pass");
}

fn typed_comparison_report(
    context: &BenchmarkApplicationContext,
    typed_context: &TypedBenchmarkContext,
    workload: &QueryWorkloadCase,
) -> TypedQueryComparisonReport {
    let plan = typed_x_range_plan(typed_context, workload);

    compare_typed_query_execution_repeated(
        &typed_context.query_index,
        &plan,
        &context.timing_config,
    )
    .expect("typed benchmark comparison should execute")
}

fn typed_x_range_plan(
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

impl TypedBenchmarkContext {
    fn from_benchmark_context(context: &BenchmarkApplicationContext) -> Self {
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

fn format_scalar_percent(value: crate::math::Scalar) -> String {
    format_percent_ratio(value as f64)
}
