//! Row-mapped FSE archive components.

mod codec;
mod file;
mod snapshot;

pub use codec::{
    FSERowMappedArchiveCodecError, decode_row_mapped_archive_snapshot,
    encode_row_mapped_archive_snapshot,
};
pub use file::{
    FSERowMappedArchiveFileError, FSERowMappedIndexArchiveError,
    load_row_mapped_index_archive_file, read_row_mapped_archive_snapshot_file,
    save_row_mapped_index_archive_file, write_row_mapped_archive_snapshot_file,
};
pub use snapshot::{
    FSELeafRowIdArchiveRecord, FSERowMappedArchiveSnapshotError, FSERowMappedIndexArchiveSnapshot,
};
