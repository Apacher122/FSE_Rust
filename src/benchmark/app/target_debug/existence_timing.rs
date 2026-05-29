//! Exact existence timing diagnostics.
//!
//! Existence is a boolean projection over the exact execution result `E(Q, F)`.
//! These diagnostics compare that output contract against owned-result and
//! count-only execution without changing the query semantics.

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
    count_query_matches_with_stats, execute_query_with_stats_and_options, query_has_match,
};

#[derive(Clone, Debug)]
struct ExistenceTimingEvidence {
    fresh_owned_elapsed: Duration,
    count_only_elapsed: Duration,
    existence_elapsed: Duration,
    owned_above_existence: Duration,
    count_only_above_existence: Duration,
    existence_speedup_vs_fresh_owned: f64,
    existence_speedup_vs_count_only: f64,
    fresh_owned_has_match: bool,
    count_only_has_match: bool,
    existence_has_match: bool,
    existence_matches_owned_presence: bool,
    existence_matches_count_only_presence: bool,
    all_existence_results_agree: bool,
}

impl ExistenceTimingEvidence {
    fn agreement_label(&self) -> &'static str {
        if self.all_existence_results_agree
            && self.existence_matches_owned_presence
            && self.existence_matches_count_only_presence
        {
            "pass"
        } else {
            "fail"
        }
    }
}

impl BenchmarkApplicationRenderer {
    pub(crate) fn append_target_workload_existence_timing_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        append_target_workload_debug_section(
            output,
            context,
            "Target workload exact existence timing",
            |output, context, workload| {
                let evidence = collect_existence_timing_evidence(context, workload);

                append_target_existence_timing_evidence(output, context, &evidence);

                // this compares output contracts, not a new query semantic
            },
        );
    }

    pub(crate) fn append_workload_existence_timing_summary_debug_output(
        &self,
        output: &mut String,
        context: &BenchmarkApplicationContext,
    ) {
        output.push_str("Workload exact existence timing summary\n");
        output.push_str("---------------------------------------\n");
        output.push_str(
            "workload | owned has match | count-only has match | existence has match | fresh owned | count-only | existence | existence speedup vs owned | existence speedup vs count-only | agreement\n",
        );

        for workload in &context.workloads {
            let evidence = collect_existence_timing_evidence(context, workload);

            output.push_str(&format!(
                "{} | {} | {} | {} | {} | {} | {} | {} | {} | {}\n",
                workload.name,
                evidence.fresh_owned_has_match,
                evidence.count_only_has_match,
                evidence.existence_has_match,
                format_duration_ascii(evidence.fresh_owned_elapsed),
                format_duration_ascii(evidence.count_only_elapsed),
                format_duration_ascii(evidence.existence_elapsed),
                format_speedup_ratio(evidence.existence_speedup_vs_fresh_owned),
                format_speedup_ratio(evidence.existence_speedup_vs_count_only),
                evidence.agreement_label(),
            ));
        }

        output.push('\n');
    }
}

fn append_target_existence_timing_evidence(
    output: &mut String,
    context: &BenchmarkApplicationContext,
    evidence: &ExistenceTimingEvidence,
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
        "count-only average elapsed",
        evidence.count_only_elapsed,
    );
    append_debug_duration_line(
        output,
        "existence average elapsed",
        evidence.existence_elapsed,
    );
    append_debug_duration_line(
        output,
        "estimated owned above existence",
        evidence.owned_above_existence,
    );
    append_debug_duration_line(
        output,
        "estimated count-only above existence",
        evidence.count_only_above_existence,
    );
    append_debug_line(
        output,
        "existence speedup vs fresh owned",
        format_speedup_ratio(evidence.existence_speedup_vs_fresh_owned),
    );
    append_debug_line(
        output,
        "existence speedup vs count-only",
        format_speedup_ratio(evidence.existence_speedup_vs_count_only),
    );
    append_debug_line(
        output,
        "fresh owned has match",
        evidence.fresh_owned_has_match,
    );
    append_debug_line(
        output,
        "count-only has match",
        evidence.count_only_has_match,
    );
    append_debug_line(output, "existence has match", evidence.existence_has_match);
    append_debug_line(
        output,
        "existence matches owned presence",
        evidence.existence_matches_owned_presence,
    );
    append_debug_line(
        output,
        "existence matches count-only presence",
        evidence.existence_matches_count_only_presence,
    );
    append_debug_line(
        output,
        "all existence results agree",
        evidence.all_existence_results_agree,
    );
}

fn collect_existence_timing_evidence(
    context: &BenchmarkApplicationContext,
    workload: &QueryWorkloadCase,
) -> ExistenceTimingEvidence {
    let timing_config = &context.timing_config;
    let query_options = context.suite_config.query_execution_options();

    let fresh_owned_timing = measure_repeated(timing_config, || {
        let report =
            execute_query_with_stats_and_options(&context.index, &workload.query, query_options);

        std::hint::black_box(!report.results.is_empty());
        std::hint::black_box(report.stats.matched_records);
    });

    let count_only_timing = measure_repeated(timing_config, || {
        let report = count_query_matches_with_stats(&context.index, &workload.query);

        std::hint::black_box(report.matched_records > 0);
        std::hint::black_box(report.matched_records);
    });

    let existence_timing = measure_repeated(timing_config, || {
        let has_match = query_has_match(&context.index, &workload.query);

        std::hint::black_box(has_match);
    });

    let fresh_owned_report =
        execute_query_with_stats_and_options(&context.index, &workload.query, query_options);
    let count_only_report = count_query_matches_with_stats(&context.index, &workload.query);
    let existence_has_match = query_has_match(&context.index, &workload.query);

    let fresh_owned_has_match = !fresh_owned_report.results.is_empty();
    let count_only_has_match = count_only_report.matched_records > 0;

    let evidence = existence_timing_evidence(
        fresh_owned_timing.average_elapsed,
        count_only_timing.average_elapsed,
        existence_timing.average_elapsed,
        fresh_owned_has_match,
        count_only_has_match,
        existence_has_match,
    );

    assert_existence_timing_equivalence(&evidence);

    evidence
}

fn existence_timing_evidence(
    fresh_owned_elapsed: Duration,
    count_only_elapsed: Duration,
    existence_elapsed: Duration,
    fresh_owned_has_match: bool,
    count_only_has_match: bool,
    existence_has_match: bool,
) -> ExistenceTimingEvidence {
    ExistenceTimingEvidence {
        fresh_owned_elapsed,
        count_only_elapsed,
        existence_elapsed,
        owned_above_existence: fresh_owned_elapsed.saturating_sub(existence_elapsed),
        count_only_above_existence: count_only_elapsed.saturating_sub(existence_elapsed),
        existence_speedup_vs_fresh_owned: duration_ratio(fresh_owned_elapsed, existence_elapsed),
        existence_speedup_vs_count_only: duration_ratio(count_only_elapsed, existence_elapsed),
        fresh_owned_has_match,
        count_only_has_match,
        existence_has_match,
        existence_matches_owned_presence: existence_has_match == fresh_owned_has_match,
        existence_matches_count_only_presence: existence_has_match == count_only_has_match,
        all_existence_results_agree: existence_has_match == fresh_owned_has_match
            && existence_has_match == count_only_has_match,
    }
}

fn assert_existence_timing_equivalence(evidence: &ExistenceTimingEvidence) {
    assert!(
        evidence.existence_matches_owned_presence,
        "target workload existence result must match owned-result presence"
    );
    assert!(
        evidence.existence_matches_count_only_presence,
        "target workload existence result must match count-only presence"
    );
    assert!(
        evidence.all_existence_results_agree,
        "target workload existence result must agree across output contracts"
    );
}
