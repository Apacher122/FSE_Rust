use std::env;

use fse_rust::benchmark::{
    BaselineRegistry, BenchmarkCsvMetadata, BenchmarkRunOverview, benchmark_usage,
    parse_benchmark_cli_config, render_benchmark_overview, render_multi_baseline_summary,
    render_named_baseline_suite_report, render_suite_report, run_multi_baseline_benchmark_suite,
    summarize_multi_baseline_aggregates, write_benchmark_csv_outputs,
};
use fse_rust::build::FSEBuilder;

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
    let run_has_multiple_baselines = cli_config.baseline_set.is_multi_baseline();

    let overview = BenchmarkRunOverview {
        dataset_records: points.len(),
        index_nodes: index.node_count(),
        workloads: workloads.len(),
        baselines: cli_config.baseline_set.selected_name_list(),
        timing_iterations: timing_config.iterations,
        max_leaf_size: config.max_leaf_size,
        max_depth: config.max_depth,
        validation,
    };

    print!("{}", render_benchmark_overview(&overview));

    let report = run_multi_baseline_benchmark_suite(
        &index,
        &points,
        &workloads,
        &timing_config,
        &registry,
        &cli_config.baseline_kinds,
    );

    if run_has_multiple_baselines {
        for baseline_report in &report.baseline_reports {
            print!(
                "{}",
                render_named_baseline_suite_report(
                    &baseline_report.baseline_name,
                    &baseline_report.report,
                )
            );
        }
    } else {
        print!(
            "{}",
            render_suite_report(&report.baseline_reports[0].report)
        );
    }

    let aggregate_summary = summarize_multi_baseline_aggregates(&report);

    if run_has_multiple_baselines {
        print!("{}", render_multi_baseline_summary(&aggregate_summary));
    }

    let metadata = BenchmarkCsvMetadata::from_overview(&overview);

    let csv_write_report = match write_benchmark_csv_outputs(
        &cli_config.csv_output,
        &metadata,
        &aggregate_summary,
        &report,
    ) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{}", error);
            std::process::exit(1);
        }
    };

    // main just prints the sucess lines now reporting owns the file details
    for status_line in csv_write_report.status_lines() {
        println!("{}", status_line);
    }
}
