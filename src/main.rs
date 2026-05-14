use fse_rust::benchmark::{
    BaselineRegistry, BenchmarkSuiteReport, MultiBaselineAggregateSummary, benchmark_usage,
    parse_benchmark_cli_config, run_benchmark_suite_with_registry,
    run_multi_baseline_benchmark_suite, summarize_multi_baseline_aggregates,
};
use fse_rust::build::FSEBuilder;
use std::env;

fn main() {
    let cli_config = match parse_benchmark_cli_config(env::args().skip(1)) {
        Ok(config) => config,
        Err(message) => {
            eprintln!("{}", message);

            if !message.contains("Usage:") {
                eprintln!();
                eprintln!("{}", benchmark_usage());
            }

            std::process::exit(1);
        }
    };

    let config = cli_config.suite_config;
    let points = config.dataset();
    let workloads = config.workloads();
    let timing_config = config.timing_config();

    let builder = FSEBuilder::new(config.build_config());
    let validated = builder.build_validated(&points);

    let index = validated.index;
    let validation = validated.validation;
    let registry = BaselineRegistry::new();

    let baseline_names: Vec<&str> = cli_config
        .baseline_kinds
        .iter()
        .map(|baseline| baseline.name())
        .collect();

    println!("FSE benchmark suite");
    println!("===================");
    println!();

    println!("Dataset records: {}", points.len());
    println!("Index nodes: {}", index.node_count());
    println!("Workloads: {}", workloads.len());
    println!("Baselines: {}", baseline_names.join(", "));
    println!("Timing iterations: {}", timing_config.iterations);
    println!("Max leaf size: {}", config.max_leaf_size);
    println!("Max build depth: {}", config.max_depth);
    println!();

    println!("Timing ratio meaning: baseline elapsed / FSE elapsed");
    println!("  above 1.0 means FSE measured faster for that run");
    println!();

    println!("Index validation: {}", validation.is_valid());
    println!(
        "  leaf cardinality valid: {}",
        validation.leaf_cardinality_valid
    );
    println!(
        "  hierarchy topology valid: {}",
        validation.hierarchy_topology_valid
    );
    println!(
        "  parent-child bounds valid: {}",
        validation.parent_child_bounds_valid
    );
    println!();

    if cli_config.baseline_kinds.len() == 1 {
        let report = run_benchmark_suite_with_registry(
            &index,
            &points,
            &workloads,
            &timing_config,
            &registry,
            cli_config.baseline_kinds[0],
        );

        print_suite_report(&report);
    } else {
        let report = run_multi_baseline_benchmark_suite(
            &index,
            &points,
            &workloads,
            &timing_config,
            &registry,
            &cli_config.baseline_kinds,
        );

        for baseline_report in &report.baseline_reports {
            println!("Baseline suite: {}", baseline_report.baseline_name);
            println!("----------------");
            print_suite_report(&baseline_report.report);
            println!();
        }

        let aggregate_summary = summarize_multi_baseline_aggregates(&report);
        print_multi_baseline_summary(&aggregate_summary);
    }
}

fn print_suite_report(report: &BenchmarkSuiteReport) {
    for (summary, pruning_report) in report.comparisons.iter().zip(&report.pruning_reports) {
        let comparison = &summary.comparison;
        let pruning = &pruning_report.pruning;

        println!("Workload: {}", summary.workload_name);
        println!("Comparison: {}", comparison.labels.comparison_label);
        println!("Stats:");
        println!("  baseline: {}", comparison.labels.baseline_label);
        println!(
            "  baseline evaluated records: {}",
            comparison.baseline_stats.evaluated_records
        );
        println!(
            "  baseline elapsed: {:?}",
            comparison.timing.baseline_elapsed
        );
        println!(
            "  baseline average elapsed: {:?}",
            comparison.repeated_timing.baseline.average_elapsed
        );
        println!(
            "  FSE visited nodes: {}",
            comparison.fse_stats.visited_nodes
        );
        println!("  FSE elapsed: {:?}", comparison.timing.fse_elapsed);
        println!(
            "  FSE average elapsed: {:?}",
            comparison.repeated_timing.fse.average_elapsed
        );
        println!(
            "  single-run timing ratio: {:.2}",
            comparison.single_run_timing_ratio
        );
        println!(
            "  average timing ratio: {:.2}",
            comparison.average_timing_ratio
        );
        println!(
            "  FSE retained leaves: {}",
            comparison.fse_stats.retained_leaves
        );
        println!(
            "  retained leaf ratio: {:.2}",
            comparison.retained_leaf_ratio
        );
        println!(
            "  leaf pruning efficiency: {:.2}",
            pruning.leaf_pruning_efficiency
        );
        println!(
            "  FSE reconstructed records: {}",
            comparison.fse_stats.reconstructed_records
        );
        println!("  candidate ratio: {:.2}", comparison.candidate_ratio);
        println!(
            "  record pruning efficiency: {:.2}",
            pruning.record_pruning_efficiency
        );
        println!(
            "  matched records: {}",
            comparison.fse_stats.matched_records
        );
        println!(
            "  avoided reconstructions: {}",
            comparison.avoided_reconstructions
        );
        println!(
            "  reconstruction avoidance ratio: {:.2}",
            comparison.reconstruction_avoidance_ratio
        );

        println!();
    }

    let aggregate = &report.aggregate;

    println!("Aggregate workload metrics");
    println!("--------------------------");
    println!(
        "total baseline evaluated records: {}",
        aggregate.total_baseline_evaluated_records
    );
    println!(
        "total FSE visited nodes: {}",
        aggregate.total_fse_visited_nodes
    );
    println!(
        "total FSE retained leaves: {}",
        aggregate.total_fse_retained_leaves
    );
    println!(
        "total FSE reconstructed records: {}",
        aggregate.total_fse_reconstructed_records
    );
    println!(
        "total FSE matched records: {}",
        aggregate.total_fse_matched_records
    );
    println!(
        "total avoided reconstructions: {}",
        aggregate.total_avoided_reconstructions
    );
    println!(
        "average reconstruction avoidance ratio: {:.2}",
        aggregate.average_reconstruction_avoidance_ratio
    );
    println!(
        "average candidate ratio: {:.2}",
        aggregate.average_candidate_ratio
    );
    println!(
        "average retained leaf ratio: {:.2}",
        aggregate.average_retained_leaf_ratio
    );
    println!(
        "weighted reconstruction avoidance ratio: {:.2}",
        aggregate.weighted_reconstruction_avoidance_ratio
    );
    println!(
        "weighted candidate ratio: {:.2}",
        aggregate.weighted_candidate_ratio
    );
    println!(
        "total baseline average elapsed: {:?}",
        aggregate.total_baseline_average_elapsed
    );
    println!(
        "total FSE average elapsed: {:?}",
        aggregate.total_fse_average_elapsed
    );
    println!(
        "mean baseline average elapsed: {:?}",
        aggregate.mean_baseline_average_elapsed
    );
    println!(
        "mean FSE average elapsed: {:?}",
        aggregate.mean_fse_average_elapsed
    );
    println!("mean timing ratio: {:.2}", aggregate.mean_timing_ratio);
    println!(
        "weighted timing ratio: {:.2}",
        aggregate.weighted_timing_ratio
    );
}

fn print_multi_baseline_summary(summary: &MultiBaselineAggregateSummary) {
    println!("Multi-baseline aggregate summary");
    println!("--------------------------------");

    for baseline in &summary.baseline_summaries {
        println!("Baseline: {}", baseline.baseline_label);
        println!("Comparison: {}", baseline.comparison_label);
        println!("  workloads: {}", baseline.workload_count);
        println!(
            "  total baseline evaluated records: {}",
            baseline.total_baseline_evaluated_records
        );
        println!(
            "  total FSE reconstructed records: {}",
            baseline.total_fse_reconstructed_records
        );
        println!(
            "  weighted reconstruction avoidance ratio: {:.2}",
            baseline.weighted_reconstruction_avoidance_ratio
        );
        println!(
            "  weighted candidate ratio: {:.2}",
            baseline.weighted_candidate_ratio
        );
        println!("  mean timing ratio: {:.2}", baseline.mean_timing_ratio);
        println!(
            "  weighted timing ratio: {:.2}",
            baseline.weighted_timing_ratio
        );
        println!(
            "  total baseline average elapsed: {:?}",
            baseline.total_baseline_average_elapsed
        );
        println!(
            "  total FSE average elapsed: {:?}",
            baseline.total_fse_average_elapsed
        );
        println!();
    }

    if let Some(highest) = summary.highest_weighted_timing_ratio() {
        println!(
            "Highest weighted timing ratio: {} ({:.2})",
            highest.baseline_label, highest.weighted_timing_ratio
        );
    }
}
