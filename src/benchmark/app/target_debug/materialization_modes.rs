//! Materialization-mode timing diagnostics.

use std::time::Duration;

use super::super::context::BenchmarkApplicationContext;
use super::super::renderer::BenchmarkApplicationRenderer;
use super::formatting::format_speedup_ratio;
use super::target::{
    append_debug_duration_line, append_debug_line, append_target_workload_debug_section,
};
use crate::benchmark::reports::output::format_duration_ascii;
use crate::benchmark::reports::{duration_ratio, measure_repeated};
use crate::benchmark::workloads::QueryWorkloadCase;
use crate::query::{
    QueryExecutionStats, count_query_matches_with_stats, execute_query_into_with_options,
    execute_query_references_with_stats, execute_query_with_stats_and_options,
    visit_query_references,
};

#[derive(Clone, Debug)]
struct MaterializationModeEvidence {
    fresh_owned_elapsed: Duration,
    reusable_owned_elapsed: Duration,
    reference_elapsed: Duration,
    visitor_elapsed: Duration,
    count_only_elapsed: Duration,
    owned_above_count_only: Duration,
    owned_above_reference: Duration,
    owned_above_visitor: Duration,
    reference_above_visitor: Duration,
    fresh_above_reusable_owned: Duration,
    count_only_speedup: f64,
    reference_speedup: f64,
    visitor_speedup: f64,
    reusable_owned_speedup: f64,
    fresh_owned_matched_records: usize,
    reusable_owned_matched_records: usize,
    reference_matched_records: usize,
    visitor_matched_records: usize,
    count_only_matched_records: usize,
    count_only_stats_match_owned: bool,
    reference_stats_match_count_only: bool,
    visitor_stats_match_reference: bool,
    reusable_owned_stats_match_owned: bool,
    all_matched_records_agree: bool,
}

impl MaterializationModeEvidence {
    fn agreement_label(&self) -> &'static str {
        if self.all_matched_records_agree
            && self.count_only_stats_match_owned
            && self.reference_stats_match_count_only
            && self.visitor_stats_match_reference
            && self.reusable_owned_stats_match_owned
        {
            "pass"
        } else {
            "fail"
        }
    }
}

impl BenchmarkApplicationRenderer {
    pub(crate) fn append_target_workload_materialization_mode_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        append_target_workload_debug_section(
            output,
            context,
            "Target workload materialization mode comparison",
            |output, context, workload| {
                let evidence = collect_materialization_mode_evidence(context, workload);

                append_target_materialization_mode_evidence(output, context, &evidence);

                // this section is evidence gathering not a performance claim
            },
        );
    }

    pub(crate) fn append_workload_materialization_mode_summary_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        output.push_str("Workload materialization mode summary\n");
        output.push_str("-------------------------------------\n");
        output.push_str(
            "workload | matched | fresh owned | reusable owned | reference | visitor | count-only | count speedup | reference speedup | visitor speedup | reusable speedup | agreement\n",
        );

        for workload in &context.workloads {
            let evidence = collect_materialization_mode_evidence(context, workload);

            output.push_str(&format!(
                "{} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {}\n",
                workload.name,
                evidence.fresh_owned_matched_records,
                format_duration_ascii(evidence.fresh_owned_elapsed),
                format_duration_ascii(evidence.reusable_owned_elapsed),
                format_duration_ascii(evidence.reference_elapsed),
                format_duration_ascii(evidence.visitor_elapsed),
                format_duration_ascii(evidence.count_only_elapsed),
                format_speedup_ratio(evidence.count_only_speedup),
                format_speedup_ratio(evidence.reference_speedup),
                format_speedup_ratio(evidence.visitor_speedup),
                format_speedup_ratio(evidence.reusable_owned_speedup),
                evidence.agreement_label(),
            ));
        }

        output.push('\n');
    }
}

fn append_target_materialization_mode_evidence(
    output: &mut String,
    context: &BenchmarkApplicationContext,
    evidence: &MaterializationModeEvidence,
) {
    append_debug_line(
        output,
        "timing iterations",
        context.timing_config.iterations,
    );
    append_debug_duration_line(
        output,
        "fresh owned average elapsed",
        evidence.fresh_owned_elapsed,
    );
    append_debug_duration_line(
        output,
        "reusable owned average elapsed",
        evidence.reusable_owned_elapsed,
    );
    append_debug_duration_line(
        output,
        "reference-result average elapsed",
        evidence.reference_elapsed,
    );
    append_debug_duration_line(
        output,
        "reference-visitor average elapsed",
        evidence.visitor_elapsed,
    );
    append_debug_duration_line(
        output,
        "count-only average elapsed",
        evidence.count_only_elapsed,
    );
    append_debug_duration_line(
        output,
        "estimated owned above count-only",
        evidence.owned_above_count_only,
    );
    append_debug_duration_line(
        output,
        "estimated owned above reference-result",
        evidence.owned_above_reference,
    );
    append_debug_duration_line(
        output,
        "estimated owned above reference-visitor",
        evidence.owned_above_visitor,
    );
    append_debug_duration_line(
        output,
        "estimated reference-result above reference-visitor",
        evidence.reference_above_visitor,
    );
    append_debug_duration_line(
        output,
        "estimated fresh above reusable owned",
        evidence.fresh_above_reusable_owned,
    );
    append_debug_line(
        output,
        "count-only speedup",
        format_speedup_ratio(evidence.count_only_speedup),
    );
    append_debug_line(
        output,
        "reference-result speedup",
        format_speedup_ratio(evidence.reference_speedup),
    );
    append_debug_line(
        output,
        "reference-visitor speedup",
        format_speedup_ratio(evidence.visitor_speedup),
    );
    append_debug_line(
        output,
        "reusable owned speedup",
        format_speedup_ratio(evidence.reusable_owned_speedup),
    );
    append_debug_line(
        output,
        "fresh owned matched records",
        evidence.fresh_owned_matched_records,
    );
    append_debug_line(
        output,
        "reusable owned matched records",
        evidence.reusable_owned_matched_records,
    );
    append_debug_line(
        output,
        "reference-result matched records",
        evidence.reference_matched_records,
    );
    append_debug_line(
        output,
        "reference-visitor matched records",
        evidence.visitor_matched_records,
    );
    append_debug_line(
        output,
        "count-only matched records",
        evidence.count_only_matched_records,
    );
    append_debug_line(
        output,
        "count-only stats match owned",
        evidence.count_only_stats_match_owned,
    );
    append_debug_line(
        output,
        "reference stats match count-only",
        evidence.reference_stats_match_count_only,
    );
    append_debug_line(
        output,
        "reference visitor stats match reference",
        evidence.visitor_stats_match_reference,
    );
    append_debug_line(
        output,
        "reusable owned stats match owned",
        evidence.reusable_owned_stats_match_owned,
    );
    append_debug_line(
        output,
        "all matched records agree",
        evidence.all_matched_records_agree,
    );
}

fn collect_materialization_mode_evidence(
    context: &BenchmarkApplicationContext,
    workload: &QueryWorkloadCase,
) -> MaterializationModeEvidence {
    let timing_config = &context.timing_config;
    let query_options = context.suite_config.query_execution_options();

    let fresh_owned_timing = measure_repeated(timing_config, || {
        let report =
            execute_query_with_stats_and_options(&context.index, &workload.query, query_options);

        std::hint::black_box(report.stats.matched_records);
        std::hint::black_box(report.results.len());
    });

    let mut reusable_owned_results = Vec::new();

    let reusable_owned_timing = measure_repeated(timing_config, || {
        let stats = execute_query_into_with_options(
            &context.index,
            &workload.query,
            query_options,
            &mut reusable_owned_results,
        );

        std::hint::black_box(stats.matched_records);
        std::hint::black_box(reusable_owned_results.len());
    });

    let reference_timing = measure_repeated(timing_config, || {
        let report = execute_query_references_with_stats(&context.index, &workload.query);

        std::hint::black_box(report.stats.matched_records);
        std::hint::black_box(report.matches.len());
    });

    let visitor_timing = measure_repeated(timing_config, || {
        let mut visited_records = 0usize;
        let stats = visit_query_references(&context.index, &workload.query, |reference| {
            visited_records += 1;
            std::hint::black_box(reference);
        });

        std::hint::black_box(stats.matched_records);
        std::hint::black_box(visited_records);
    });

    let count_only_timing = measure_repeated(timing_config, || {
        let report = count_query_matches_with_stats(&context.index, &workload.query);

        std::hint::black_box(report.matched_records);
    });

    let fresh_owned_report =
        execute_query_with_stats_and_options(&context.index, &workload.query, query_options);
    let mut reusable_owned_check_results = Vec::new();
    let reusable_owned_stats = execute_query_into_with_options(
        &context.index,
        &workload.query,
        query_options,
        &mut reusable_owned_check_results,
    );
    let reference_report = execute_query_references_with_stats(&context.index, &workload.query);
    let mut visitor_matched_records = 0usize;
    let visitor_stats = visit_query_references(&context.index, &workload.query, |reference| {
        visitor_matched_records += 1;
        std::hint::black_box(reference);
    });
    let count_report = count_query_matches_with_stats(&context.index, &workload.query);

    let evidence = materialization_mode_evidence(
        fresh_owned_timing.average_elapsed,
        reusable_owned_timing.average_elapsed,
        reference_timing.average_elapsed,
        visitor_timing.average_elapsed,
        count_only_timing.average_elapsed,
        fresh_owned_report.stats,
        reusable_owned_stats,
        reference_report.stats,
        reference_report.matches.len(),
        visitor_stats,
        visitor_matched_records,
        count_report.stats,
        count_report.matched_records,
    );

    assert_materialization_mode_equivalence(&evidence);

    evidence
}

fn materialization_mode_evidence(
    fresh_owned_elapsed: Duration,
    reusable_owned_elapsed: Duration,
    reference_elapsed: Duration,
    visitor_elapsed: Duration,
    count_only_elapsed: Duration,
    fresh_owned_stats: QueryExecutionStats,
    reusable_owned_stats: QueryExecutionStats,
    reference_stats: QueryExecutionStats,
    reference_matched_records: usize,
    visitor_stats: QueryExecutionStats,
    visitor_matched_records: usize,
    count_only_stats: QueryExecutionStats,
    count_only_matched_records: usize,
) -> MaterializationModeEvidence {
    let owned_above_count_only = fresh_owned_elapsed.saturating_sub(count_only_elapsed);
    let owned_above_reference = fresh_owned_elapsed.saturating_sub(reference_elapsed);
    let owned_above_visitor = fresh_owned_elapsed.saturating_sub(visitor_elapsed);
    let reference_above_visitor = reference_elapsed.saturating_sub(visitor_elapsed);
    let fresh_above_reusable_owned = fresh_owned_elapsed.saturating_sub(reusable_owned_elapsed);

    let count_only_speedup = duration_ratio(fresh_owned_elapsed, count_only_elapsed);
    let reference_speedup = duration_ratio(fresh_owned_elapsed, reference_elapsed);
    let visitor_speedup = duration_ratio(fresh_owned_elapsed, visitor_elapsed);
    let reusable_owned_speedup = duration_ratio(fresh_owned_elapsed, reusable_owned_elapsed);

    let count_only_stats_match_owned = count_only_stats == fresh_owned_stats;
    let reference_stats_match_count_only = reference_stats == count_only_stats;
    let visitor_stats_match_reference = visitor_stats == reference_stats;
    let reusable_owned_stats_match_owned = reusable_owned_stats == fresh_owned_stats;
    let all_matched_records_agree = fresh_owned_stats.matched_records
        == reusable_owned_stats.matched_records
        && fresh_owned_stats.matched_records == reference_matched_records
        && fresh_owned_stats.matched_records == visitor_matched_records
        && fresh_owned_stats.matched_records == count_only_matched_records;

    MaterializationModeEvidence {
        fresh_owned_elapsed,
        reusable_owned_elapsed,
        reference_elapsed,
        visitor_elapsed,
        count_only_elapsed,
        owned_above_count_only,
        owned_above_reference,
        owned_above_visitor,
        reference_above_visitor,
        fresh_above_reusable_owned,
        count_only_speedup,
        reference_speedup,
        visitor_speedup,
        reusable_owned_speedup,
        fresh_owned_matched_records: fresh_owned_stats.matched_records,
        reusable_owned_matched_records: reusable_owned_stats.matched_records,
        reference_matched_records,
        visitor_matched_records,
        count_only_matched_records,
        count_only_stats_match_owned,
        reference_stats_match_count_only,
        visitor_stats_match_reference,
        reusable_owned_stats_match_owned,
        all_matched_records_agree,
    }
}

fn assert_materialization_mode_equivalence(evidence: &MaterializationModeEvidence) {
    assert!(
        evidence.all_matched_records_agree,
        "target workload materialization modes must agree on exact match count"
    );
    assert!(
        evidence.count_only_stats_match_owned,
        "target workload count-only stats must match owned-result stats"
    );
    assert!(
        evidence.reference_stats_match_count_only,
        "target workload reference-result stats must match count-only stats"
    );
    assert!(
        evidence.visitor_stats_match_reference,
        "target workload reference visitor stats must match reference-result stats"
    );
    assert!(
        evidence.reusable_owned_stats_match_owned,
        "target workload reusable owned stats must match fresh owned stats"
    );
}
