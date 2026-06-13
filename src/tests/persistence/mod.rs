mod codec_tests;
mod file_tests;
mod index_file_tests;
mod manifest_tests;
mod operation_tests;
mod payload_tests;
mod row_mapping_codec_tests;
mod row_mapping_file_tests;
mod row_mapping_tests;
mod snapshot_tests;
mod typed;

use crate::persistence::{FSEArchivePayloadKind, encode_archive_payload};

fn corrupted_archive_payload(kind: FSEArchivePayloadKind) -> Vec<u8> {
    let mut bytes = encode_archive_payload(kind, &[1_u8, 2, 3, 4]);
    let last = bytes.last_mut().unwrap();
    *last ^= 0xff;

    bytes
}
