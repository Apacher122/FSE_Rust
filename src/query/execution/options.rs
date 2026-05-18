//! Query execution configuration.
//!
//! This module defines the runtime strategy options used by the FSE query
//! engine. These options control how retained partitions are executed after
//! geometric traversal has identified the candidate leaf set.

/// Default retained-leaf threshold required before parallel execution uses Rayon.
///
/// # Runtime Role
///
/// Parallel retained-leaf execution has scheduling overhead. This threshold
/// keeps small retained-leaf batches on the deterministic serial path while
/// still allowing larger batches to use Rayon.
pub const DEFAULT_PARALLEL_MIN_RETAINED_LEAVES: usize = 4;

/// Execution strategy used by the query runtime.
///
/// # Runtime Role
///
/// `QueryExecutionMode` makes the retained-partition execution strategy explicit.
/// Serial execution processes retained partitions one at a time. Parallel
/// execution evaluates retained leaf partitions independently before merging
/// their local reports in deterministic retained-leaf order.
///
/// # Formal Reference
///
/// This controls how retained partitions are processed after geometric
/// selection. It does not change the required semantic order:
///
/// `Geometry -> Reconstruction -> Logic`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryExecutionMode {
    /// Retained partitions are reconstructed and evaluated one at a time.
    Serial,

    /// Retained partitions are reconstructed and evaluated independently using Rayon.
    Parallel,
}

impl Default for QueryExecutionMode {
    fn default() -> Self {
        Self::Serial
    }
}

/// Options controlling query execution behavior.
///
/// # Runtime Role
///
/// `QueryExecutionOptions` provides a stable place to configure execution
/// strategy without changing the query API every time the runtime gains a new
/// execution mode.
///
/// The default is deterministic serial execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QueryExecutionOptions {
    /// Retained-partition execution strategy.
    pub mode: QueryExecutionMode,

    /// Minimum retained-leaf count required before parallel mode uses Rayon.
    ///
    /// This value is ignored by serial mode.
    pub parallel_min_retained_leaves: usize,
}

impl QueryExecutionOptions {
    /// Creates options for deterministic serial query execution.
    pub fn serial() -> Self {
        Self {
            mode: QueryExecutionMode::Serial,
            parallel_min_retained_leaves: DEFAULT_PARALLEL_MIN_RETAINED_LEAVES,
        }
    }

    /// Creates options for parallel retained-partition query execution.
    ///
    /// # Runtime Role
    ///
    /// Parallel execution evaluates retained leaf partitions independently while
    /// preserving deterministic final result ordering through ordered report
    /// collection and merge. Small retained-leaf batches fall back to serial
    /// execution based on `parallel_min_retained_leaves`.
    pub fn parallel() -> Self {
        Self {
            mode: QueryExecutionMode::Parallel,
            parallel_min_retained_leaves: DEFAULT_PARALLEL_MIN_RETAINED_LEAVES,
        }
    }

    /// Returns a copy of the options with a new parallel retained-leaf threshold.
    ///
    /// # Runtime Role
    ///
    /// This allows benchmarks and tests to tune the point where parallel mode
    /// starts using Rayon without changing the selected execution mode.
    pub fn with_parallel_min_retained_leaves(mut self, threshold: usize) -> Self {
        // no magic here just move the cutoff around
        self.parallel_min_retained_leaves = threshold;
        self
    }
}

impl Default for QueryExecutionOptions {
    fn default() -> Self {
        Self::serial()
    }
}
