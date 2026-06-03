//! Global FSE index hierarchy storage.

mod index;
mod leaf_shape;

pub use index::{FSEIndex, FSEIndexError};
pub use leaf_shape::LeafReconstructionShape;
