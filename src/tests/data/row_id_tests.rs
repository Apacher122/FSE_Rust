use std::collections::HashSet;

use crate::data::RowId;

#[test]
fn row_id_preserves_numeric_value() {
    let row_id = RowId::new(42);

    assert_eq!(row_id.value(), 42);
}

#[test]
fn row_id_supports_identity_comparison() {
    assert_eq!(RowId::new(7), RowId::new(7));
    assert_ne!(RowId::new(7), RowId::new(8));
}

#[test]
fn row_id_supports_hash_lookup() {
    let mut row_ids = HashSet::new();
    row_ids.insert(RowId::new(1));
    row_ids.insert(RowId::new(2));

    assert!(row_ids.contains(&RowId::new(1)));
    assert!(!row_ids.contains(&RowId::new(3)));
}

#[test]
fn row_id_supports_deterministic_ordering() {
    let mut row_ids = vec![RowId::new(3), RowId::new(1), RowId::new(2)];
    row_ids.sort();

    assert_eq!(row_ids, vec![RowId::new(1), RowId::new(2), RowId::new(3)]);
}
