//! Benchmark CLI usage text.

/// Returns benchmark CLI usage text.
pub fn benchmark_usage() -> String {
    [
        "Usage:",
        "  cargo run --release -- [options]",
        "",
        "Options:",
        "  --baseline <flat_scan|kd_tree|r_tree>",
        "  --all-baselines",
        "  --dataset <small|large>",
        "  --iterations <N>",
        "  --target-leaf-size <N>",
        "  --max-leaf-size <N>",
        "  --max-depth <N>",
        "  --fse-execution <serial|parallel>",
        "  --fse-parallel-min-leaves <N>",
        "  --csv-summary <PATH>",
        "  --csv <PATH>",
        "  --csv-workloads <PATH>",
        "  --csv-low-selectivity-gap <PATH>",
        "  --typed-query-index-archive <PATH>",
        "  --debug-report",
    ]
    .join("\n")
}
