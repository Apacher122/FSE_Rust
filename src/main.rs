use std::env;
use std::io;

use fse_rust::benchmark::{
    BenchmarkApplicationOutputWriter, benchmark_usage, parse_benchmark_cli_config,
    run_benchmark_application,
};

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

    let output = match run_benchmark_application(cli_config) {
        Ok(output) => output,
        Err(error) => {
            eprintln!("{}", error);
            std::process::exit(1);
        }
    };

    let output_writer = BenchmarkApplicationOutputWriter::new();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    if let Err(error) = output_writer.write(&output, &mut stdout) {
        eprintln!("failed to write benchmark output: {}", error);
        std::process::exit(1);
    }
}
