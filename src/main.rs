use fse_rust::benchmark::{
    BaselineRegistry, BenchmarkRunOverview, benchmark_usage, parse_benchmark_cli_config,
    render_benchmark_overview, render_multi_baseline_summary, render_named_baseline_suite_report,
    render_suite_report, run_benchmark_suite_with_registry, run_multi_baseline_benchmark_suite,
    summarize_multi_baseline_aggregates,
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

    let overview = BenchmarkRunOverview {
        dataset_records: points.len(),
        index_nodes: index.node_count(),
        workloads: workloads.len(),
        baselines: baseline_names.join(", "),
        timing_iterations: timing_config.iterations,
        max_leaf_size: config.max_leaf_size,
        max_depth: config.max_depth,
        validation,
    };

    print!("{}", render_benchmark_overview(&overview));

    if cli_config.baseline_kinds.len() == 1 {
        let report = run_benchmark_suite_with_registry(
            &index,
            &points,
            &workloads,
            &timing_config,
            &registry,
            cli_config.baseline_kinds[0],
        );

        print!("{}", render_suite_report(&report));
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
            print!(
                "{}",
                render_named_baseline_suite_report(
                    &baseline_report.baseline_name,
                    &baseline_report.report,
                )
            );
        }

        let aggregate_summary = summarize_multi_baseline_aggregates(&report);
        print!("{}", render_multi_baseline_summary(&aggregate_summary));
    }
}
