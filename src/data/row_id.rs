//! Stable row identity metadata.

/// Stable logical row identifier.
///
/// # Runtime Role
///
/// `RowId` identifies a typed record independently of its physical partition
/// placement. Encoded coordinates and query results can use this identifier to
/// map reconstructed data back to the original logical row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RowId(u64);

impl RowId {
    /// Creates a row identifier from its numeric value.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the numeric row identifier value.
    pub fn value(self) -> u64 {
        self.0
    }
}
