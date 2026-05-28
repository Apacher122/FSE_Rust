//! Bounding box metrics.

use crate::math::Scalar;

use super::BoundingBox;

impl BoundingBox {
    /// Returns the volume of the bounding box.
    ///
    /// # Runtime Role
    ///
    /// Volume is used by structural density metrics to estimate how tightly a
    /// partition's bounded support represents its contained records.
    ///
    /// # Formal Reference
    ///
    /// This corresponds to $\operatorname{Vol}(B_k)$.
    ///
    /// # Notes
    ///
    /// Degenerate dimensions with zero width produce zero volume.
    pub fn volume(&self) -> Scalar {
        let mut volume = 1.0;

        for dimension in 0..self.dimensions() {
            let width = self.max[dimension] - self.min[dimension];

            // dont let inverted bounds look like real volume
            if width < 0.0 {
                return 0.0;
            }

            volume *= width;
        }

        volume
    }
}
