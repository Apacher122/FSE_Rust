use fse_rust::benchmark::{
    aggregate_workload_metrics, clustered_points_2d, clustered_workload_cases,
    summarize_workload_comparisons,
};
use fse_rust::build::{BuildConfig, FSEBuilder};

fn main() {
    let points = clustered_points_2d();

    let builder = FSEBuilder::new(BuildConfig::new(8, 8));
    let validated = builder.build_validated(&points);

    let index = validated.index;
    let validation = validated.validation;

    let workloads = clustered_workload_cases();
    let summaries = summarize_workload_comparisons(&index, &points, &workloads);
    let aggregate = aggregate_workload_metrics(&summaries);

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

    println!("FSE query workload demo");
    println!("=======================");
    println!();

    println!("Dataset records: {}", points.len());
    println!("Index nodes: {}", index.node_count());
    println!("Workloads: {}", aggregate.workload_count);
    println!();

    for summary in &summaries {
        let comparison = &summary.comparison;

        println!("Workload: {}", summary.workload_name);
        println!("Stats:");
        println!(
            "  flat scan evaluated records: {}",
            comparison.scan_stats.evaluated_records
        );
        println!(
            "  FSE visited nodes: {}",
            comparison.fse_stats.visited_nodes
        );
        println!(
            "  FSE retained leaves: {}",
            comparison.fse_stats.retained_leaves
        );
        println!(
            "  FSE reconstructed records: {}",
            comparison.fse_stats.reconstructed_records
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
}
