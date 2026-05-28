//! Benchmark suite runner.

mod multi;
mod report;
mod suite;

pub use multi::{
    run_multi_baseline_benchmark_suite, run_multi_baseline_benchmark_suite_with_options,
};

pub use report::{
    BaselineBenchmarkSuiteReport, BenchmarkSuiteReport, MultiBaselineBenchmarkSuiteReport,
    WorkloadPruningReport,
};

pub use suite::{
    run_benchmark_suite, run_benchmark_suite_repeated, run_benchmark_suite_repeated_with_options,
    run_benchmark_suite_with_registry, run_benchmark_suite_with_registry_and_options,
};
