//! Command-line parsing for benchmark configuration.

mod parsing;
mod state;
mod types;
mod usage;

pub use parsing::{parse_benchmark_cli_config, parse_benchmark_config};
pub use types::{BenchmarkCliConfig, BenchmarkTerminalOutputMode};
pub use usage::benchmark_usage;
