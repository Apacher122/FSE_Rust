use crate::data::{FSEFieldType, FSEValue};
use crate::encoding::{
    BooleanEncoder, FSEEncodingError, FSEFieldEncoder, FloatEncoder, IntegerEncoder,
    TimestampMillisEncoder,
};

#[test]
fn integer_encoder_encodes_integer_values() {
    let encoder = IntegerEncoder;
    let coordinates = encoder
        .encode_value(&FSEValue::Integer(42))
        .expect("integer value should encode");

    assert_eq!(encoder.field_type(), FSEFieldType::Integer);
    assert_eq!(encoder.output_dimensions(), 1);
    assert_eq!(coordinates.values(), &[42.0]);
}

#[test]
fn integer_encoder_preserves_order_for_representative_values() {
    let encoder = IntegerEncoder;

    let low = encoder
        .encode_value(&FSEValue::Integer(-10))
        .expect("integer value should encode");
    let high = encoder
        .encode_value(&FSEValue::Integer(10))
        .expect("integer value should encode");

    assert!(low.values()[0] < high.values()[0]);
}

#[test]
fn float_encoder_encodes_finite_float_values() {
    let encoder = FloatEncoder;
    let coordinates = encoder
        .encode_value(&FSEValue::Float(1.5))
        .expect("finite float should encode");

    assert_eq!(encoder.field_type(), FSEFieldType::Float);
    assert_eq!(encoder.output_dimensions(), 1);
    assert_eq!(coordinates.values(), &[1.5]);
}

#[test]
fn float_encoder_rejects_non_finite_float_values() {
    let encoder = FloatEncoder;
    let error = encoder
        .encode_value(&FSEValue::Float(f64::NAN))
        .expect_err("non-finite float should be rejected");

    assert_eq!(
        error,
        FSEEncodingError::UnsupportedValue {
            reason: "float encoder requires finite values".to_string(),
        }
    );
    assert_eq!(error.to_string(), "float encoder requires finite values");
}

#[test]
fn timestamp_encoder_encodes_timestamp_values() {
    let encoder = TimestampMillisEncoder;
    let coordinates = encoder
        .encode_value(&FSEValue::TimestampMillis(1_735_689_600_000))
        .expect("timestamp value should encode");

    assert_eq!(encoder.field_type(), FSEFieldType::TimestampMillis);
    assert_eq!(encoder.output_dimensions(), 1);
    assert_eq!(coordinates.values(), &[1_735_689_600_000.0_f32]);
}

#[test]
fn timestamp_encoder_preserves_order_for_representative_values() {
    let encoder = TimestampMillisEncoder;

    let earlier = encoder
        .encode_value(&FSEValue::TimestampMillis(1_735_689_600_000))
        .expect("timestamp value should encode");
    let later = encoder
        .encode_value(&FSEValue::TimestampMillis(1_735_689_700_000))
        .expect("timestamp value should encode");

    assert!(earlier.values()[0] < later.values()[0]);
}

#[test]
fn boolean_encoder_encodes_false_and_true_values() {
    let encoder = BooleanEncoder;

    let false_value = encoder
        .encode_value(&FSEValue::Boolean(false))
        .expect("boolean value should encode");
    let true_value = encoder
        .encode_value(&FSEValue::Boolean(true))
        .expect("boolean value should encode");

    assert_eq!(encoder.field_type(), FSEFieldType::Boolean);
    assert_eq!(encoder.output_dimensions(), 1);
    assert_eq!(false_value.values(), &[0.0]);
    assert_eq!(true_value.values(), &[1.0]);
}

#[test]
fn numeric_encoders_reject_null_values() {
    assert_eq!(
        IntegerEncoder
            .encode_value(&FSEValue::Null)
            .expect_err("null should be rejected"),
        FSEEncodingError::NullValue
    );
    assert_eq!(
        FloatEncoder
            .encode_value(&FSEValue::Null)
            .expect_err("null should be rejected"),
        FSEEncodingError::NullValue
    );
    assert_eq!(
        TimestampMillisEncoder
            .encode_value(&FSEValue::Null)
            .expect_err("null should be rejected"),
        FSEEncodingError::NullValue
    );
    assert_eq!(
        BooleanEncoder
            .encode_value(&FSEValue::Null)
            .expect_err("null should be rejected"),
        FSEEncodingError::NullValue
    );
}

#[test]
fn numeric_encoders_report_type_mismatch() {
    let error = IntegerEncoder
        .encode_value(&FSEValue::Text("42".to_string()))
        .expect_err("text should be rejected by integer encoder");

    assert_eq!(
        error,
        FSEEncodingError::FieldTypeMismatch {
            expected: FSEFieldType::Integer,
            actual: FSEFieldType::Text,
        }
    );
    assert_eq!(error.to_string(), "encoder expected Integer but found Text");
}
