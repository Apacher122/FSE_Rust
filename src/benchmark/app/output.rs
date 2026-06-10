//! Output writing for completed benchmark application runs.

use std::io::{self, Write};

/// Completed benchmark application output.
///
/// # Runtime Role
///
/// `BenchmarkApplicationOutput` contains everything the binary entrypoint needs
/// to print after a benchmark run completes. The benchmark module builds the
/// output, while `main` only writes it to stdout.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BenchmarkApplicationOutput {
    /// Fully rendered benchmark terminal output.
    pub terminal_output: String,

    /// Status lines for files written during the run.
    pub status_lines: Vec<String>,
}

impl BenchmarkApplicationOutput {
    /// Creates benchmark application output from rendered terminal text and status lines.
    pub fn new(terminal_output: String, status_lines: Vec<String>) -> Self {
        Self {
            terminal_output,
            status_lines,
        }
    }

    /// Returns whether no output was produced.
    pub fn is_empty(&self) -> bool {
        self.terminal_output.is_empty() && self.status_lines.is_empty()
    }
}

/// Writer for completed benchmark application output.
///
/// # Runtime Role
///
/// `BenchmarkApplicationOutputWriter` owns final output emission for the binary
/// entrypoint. It writes rendered terminal output first, followed by file status
/// lines in the same order used by previous `main` output.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BenchmarkApplicationOutputWriter;

impl BenchmarkApplicationOutputWriter {
    /// Creates a benchmark application output writer.
    pub fn new() -> Self {
        Self
    }

    /// Writes completed benchmark application output to the supplied writer.
    pub fn write<W>(&self, output: &BenchmarkApplicationOutput, writer: &mut W) -> io::Result<()>
    where
        W: Write,
    {
        // terminal output already has its own spacing from the renderer
        writer.write_all(output.terminal_output.as_bytes())?;

        for status_line in &output.status_lines {
            // status lines keep println style newlines here
            writeln!(writer, "{}", status_line)?;
        }

        Ok(())
    }
}
