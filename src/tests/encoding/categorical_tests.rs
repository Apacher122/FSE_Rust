use crate::data::{FSEFieldType, FSEValue};
use crate::encoding::{
    CategoricalDictionaryEncoder, CategoricalDictionaryError, FSEEncodingError, FSEFieldEncoder,
};

#[test]
fn categorical_dictionary_encoder_assigns_stable_codes() {
    let encoder = CategoricalDictionaryEncoder::new(vec![
        "open".to_string(),
        "closed".to_string(),
        "pending".to_string(),
    ]);

    assert_eq!(encoder.field_type(), FSEFieldType::Category);
    assert_eq!(encoder.output_dimensions(), 1);
    assert_eq!(encoder.categories(), &["open", "closed", "pending"]);
    assert_eq!(encoder.code_for_category("open"), Some(0));
    assert_eq!(encoder.code_for_category("closed"), Some(1));
    assert_eq!(encoder.code_for_category("missing"), None);
}

#[test]
fn categorical_dictionary_encoder_encodes_known_categories() {
    let encoder = CategoricalDictionaryEncoder::new(vec!["open".to_string(), "closed".to_string()]);

    let coordinates = encoder
        .encode_value(&FSEValue::Category("closed".to_string()))
        .expect("known category should encode");

    assert_eq!(coordinates.values(), &[1.0]);
}

#[test]
fn categorical_dictionary_encoder_rejects_unknown_categories() {
    let encoder = CategoricalDictionaryEncoder::new(vec!["open".to_string(), "closed".to_string()]);

    let error = encoder
        .encode_value(&FSEValue::Category("pending".to_string()))
        .expect_err("unknown category should be rejected");

    assert_eq!(
        error,
        FSEEncodingError::UnsupportedValue {
            reason: "category 'pending' is not in dictionary".to_string(),
        }
    );
    assert_eq!(error.to_string(), "category 'pending' is not in dictionary");
}

#[test]
fn categorical_dictionary_encoder_rejects_null_values() {
    let encoder = CategoricalDictionaryEncoder::new(vec!["open".to_string()]);

    let error = encoder
        .encode_value(&FSEValue::Null)
        .expect_err("null should be rejected");

    assert_eq!(error, FSEEncodingError::NullValue);
}

#[test]
fn categorical_dictionary_encoder_reports_type_mismatch() {
    let encoder = CategoricalDictionaryEncoder::new(vec!["open".to_string()]);

    let error = encoder
        .encode_value(&FSEValue::Text("open".to_string()))
        .expect_err("text should be rejected by category encoder");

    assert_eq!(
        error,
        FSEEncodingError::FieldTypeMismatch {
            expected: FSEFieldType::Category,
            actual: FSEFieldType::Text,
        }
    );
    assert_eq!(
        error.to_string(),
        "encoder expected Category but found Text"
    );
}

#[test]
fn checked_categorical_dictionary_reports_empty_dictionary() {
    let error = CategoricalDictionaryEncoder::try_new(Vec::new())
        .expect_err("empty dictionary should be rejected");

    assert_eq!(error, CategoricalDictionaryError::EmptyDictionary);
    assert_eq!(
        error.to_string(),
        "categorical dictionary must contain at least one category"
    );
}

#[test]
fn checked_categorical_dictionary_reports_empty_category() {
    let error = CategoricalDictionaryEncoder::try_new(vec!["open".to_string(), String::new()])
        .expect_err("empty category should be rejected");

    assert_eq!(
        error,
        CategoricalDictionaryError::EmptyCategory { index: 1 }
    );
    assert_eq!(error.to_string(), "category 1 must not be empty");
}

#[test]
fn checked_categorical_dictionary_reports_duplicate_category() {
    let error = CategoricalDictionaryEncoder::try_new(vec![
        "open".to_string(),
        "closed".to_string(),
        "open".to_string(),
    ])
    .expect_err("duplicate category should be rejected");

    assert_eq!(
        error,
        CategoricalDictionaryError::DuplicateCategory {
            category: "open".to_string(),
        }
    );
    assert_eq!(error.to_string(), "category 'open' appears more than once");
}

#[test]
#[should_panic(expected = "categorical dictionary must contain at least one category")]
fn categorical_dictionary_rejects_empty_dictionary() {
    let _encoder = CategoricalDictionaryEncoder::new(Vec::new());
}
