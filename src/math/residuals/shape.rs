//! Residual block shape and accessor helpers.

use super::ResidualBlock;

impl ResidualBlock {
    /// Returns the total number of dimensions tracked by this residual block.
    pub fn dimensions(&self) -> usize {
        self.dimensions.len()
    }

    /// Returns the number of individual records represented within the block.
    pub fn cardinality(&self) -> usize {
        self.dimensions.first().map_or(0, Vec::len)
    }

    /// Returns true when all residual dimensions contain the same number of rows.
    ///
    /// # Runtime Role
    ///
    /// This is useful for validation paths that need to inspect residual storage
    /// without constructing a new block.
    pub fn has_consistent_shape(&self) -> bool {
        let Some(first_dimension) = self.dimensions.first() else {
            return true;
        };

        let expected_rows = first_dimension.len();

        self.dimensions
            .iter()
            .all(|dimension| dimension.len() == expected_rows)
    }

    /// Returns the row count stored by each residual dimension.
    ///
    /// # Runtime Role
    ///
    /// This supports diagnostics and tests for malformed residual storage.
    pub fn dimension_lengths(&self) -> Vec<usize> {
        self.dimensions.iter().map(Vec::len).collect()
    }

    /// Checks if the residual block is completely empty.
    pub fn is_empty(&self) -> bool {
        self.cardinality() == 0
    }
}
