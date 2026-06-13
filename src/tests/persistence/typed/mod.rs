mod query;
mod record_batch;
mod tombstone;

use crate::persistence::{FSEArchivePayloadKind, encode_archive_payload};

fn corrupted_archive_payload(kind: FSEArchivePayloadKind) -> Vec<u8> {
    let mut bytes = encode_archive_payload(kind, &[1_u8, 2, 3, 4]);
    let last = bytes.last_mut().unwrap();
    *last ^= 0xff;

    bytes
}
