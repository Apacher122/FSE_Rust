use crate::build::{BuildConfig, BuildInputError, FSEBuilder};
use crate::data::RowId;
use crate::encoding::EncodedRecordBatch;
use crate::math::Vector;

#[test]
fn row_mapped_encoded_build_preserves_single_leaf_row_ids() {
    let encoded = EncodedRecordBatch::new(
        vec![RowId::new(10), RowId::new(11)],
        vec![
            Vector::new(vec![42.0, 41.881]),
            Vector::new(vec![43.0, 41.882]),
        ],
    );
    let builder = FSEBuilder::new(BuildConfig::new(4, 8));

    let mapped = builder
        .try_build_row_mapped_encoded_batch(&encoded)
        .expect("valid encoded batch should build a row-mapped index");

    assert_eq!(mapped.index().node_count(), 1);
    assert_eq!(
        mapped.leaf_row_ids(0),
        Some([RowId::new(10), RowId::new(11)].as_slice())
    );
    assert_eq!(mapped.row_id_for_leaf_row(0, 0), Some(RowId::new(10)));
    assert_eq!(mapped.row_id_for_leaf_row(0, 1), Some(RowId::new(11)));
}

#[test]
fn row_mapped_encoded_build_preserves_row_ids_across_recursive_splits() {
    let encoded = EncodedRecordBatch::new(
        vec![
            RowId::new(10),
            RowId::new(11),
            RowId::new(12),
            RowId::new(13),
        ],
        vec![
            Vector::new(vec![0.0, 0.0]),
            Vector::new(vec![1.0, 0.0]),
            Vector::new(vec![50.0, 0.0]),
            Vector::new(vec![51.0, 0.0]),
        ],
    );
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));

    let mapped = builder
        .try_build_row_mapped_encoded_batch(&encoded)
        .expect("valid encoded batch should build a row-mapped index");

    assert_eq!(mapped.index().node_count(), 3);
    assert_eq!(
        mapped.leaf_row_ids(1),
        Some([RowId::new(10), RowId::new(11)].as_slice())
    );
    assert_eq!(
        mapped.leaf_row_ids(2),
        Some([RowId::new(12), RowId::new(13)].as_slice())
    );
    assert_eq!(mapped.row_id_for_leaf_row(1, 1), Some(RowId::new(11)));
    assert_eq!(mapped.row_id_for_leaf_row(2, 0), Some(RowId::new(12)));
}

#[test]
fn row_mapped_index_returns_none_for_internal_or_missing_rows() {
    let encoded = EncodedRecordBatch::new(
        vec![
            RowId::new(10),
            RowId::new(11),
            RowId::new(12),
            RowId::new(13),
        ],
        vec![
            Vector::new(vec![0.0, 0.0]),
            Vector::new(vec![1.0, 0.0]),
            Vector::new(vec![50.0, 0.0]),
            Vector::new(vec![51.0, 0.0]),
        ],
    );
    let builder = FSEBuilder::new(BuildConfig::new(2, 8));
    let mapped = builder
        .try_build_row_mapped_encoded_batch(&encoded)
        .expect("valid encoded batch should build a row-mapped index");

    assert_eq!(mapped.leaf_row_ids(0), None);
    assert_eq!(mapped.leaf_row_ids(99), None);
    assert_eq!(mapped.row_id_for_leaf_row(1, 99), None);
}

#[test]
fn row_mapped_encoded_build_reports_empty_batch_input() {
    let encoded = EncodedRecordBatch::new(Vec::new(), Vec::new());
    let builder = FSEBuilder::new(BuildConfig::new(4, 8));

    let error = builder
        .try_build_row_mapped_encoded_batch(&encoded)
        .expect_err("empty encoded batch should be rejected");

    assert_eq!(error, BuildInputError::EmptyPointSet);
}

#[test]
#[should_panic(expected = "cannot build an index from empty points")]
fn row_mapped_encoded_build_panics_for_empty_batch() {
    let encoded = EncodedRecordBatch::new(Vec::new(), Vec::new());
    let builder = FSEBuilder::new(BuildConfig::new(4, 8));

    let _mapped = builder.build_row_mapped_encoded_batch(&encoded);
}
