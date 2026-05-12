use fse_rust::benchmark::{clustered_points_2d, clustered_workload_cases, run_benchmark_suite};
use fse_rust::build::{BuildConfig, FSEBuilder};

fn main() {
    let points = clustered_points_2d();

    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let validated = builder.build_validated(&points);

    let index = validated.index;
    let validation = validated.validation;

    let workloads = clustered_workload_cases();
    let report = run_benchmark_suite(&index, &points, &workloads);

    println!("FSE benchmark suite");
    println!("===================");
    println!();

    println!("Dataset records: {}", points.len());
    println!("Index nodes: {}", index.node_count());
    println!("Workloads: {}", report.aggregate.workload_count);
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

    for (summary, pruning_report) in report.comparisons.iter().zip(&report.pruning_reports) {
        let comparison = &summary.comparison;
        let pruning = &pruning_report.pruning;

        println!("Workload: {}", summary.workload_name);
        println!("Stats:");
        println!(
            "  flat scan evaluated records: {}",
            comparison.scan_stats.evaluated_records
        );
        println!(
            "  flat scan elapsed: {:?}",
            comparison.timing.flat_scan_elapsed
        );
        println!(
            "  FSE visited nodes: {}",
            comparison.fse_stats.visited_nodes
        );
        println!("  FSE elapsed: {:?}", comparison.timing.fse_elapsed);
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
        "total flat scan evaluated records: {}",
        aggregate.total_scan_evaluated_records
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
}
