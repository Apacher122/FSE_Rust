//! Row-mapped FSE archive components.

mod codec;
mod snapshot;

pub use codec::{
    FSERowMappedArchiveCodecError, decode_row_mapped_archive_snapshot,
    encode_row_mapped_archive_snapshot,
};
pub use snapshot::{
    FSELeafRowIdArchiveRecord, FSERowMappedArchiveSnapshotError, FSERowMappedIndexArchiveSnapshot,
};
