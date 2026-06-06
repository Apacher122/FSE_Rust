//! Typed predicate evaluation against records.

use crate::data::{FSERecord, FSEValue};

use super::predicate::{ValidatedFSEPredicate, ValidatedFSEPredicateOperator};

/// Evaluates a validated typed predicate against a typed record.
///
/// # Runtime Role
///
/// This function is the exact post-reconstruction predicate check for
/// FSE-native records. Geometric query regions can prune candidates before this
/// step, but retained records still use typed logical evaluation here.
pub fn evaluate_typed_predicate(record: &FSERecord, predicate: &ValidatedFSEPredicate) -> bool {
    let Some(value) = record.value(predicate.field()) else {
        return false;
    };

    match predicate.operator() {
        ValidatedFSEPredicateOperator::Equal(expected) => value == expected,
        ValidatedFSEPredicateOperator::Range { min, max } => value_in_closed_range(value, min, max),
    }
}

fn value_in_closed_range(value: &FSEValue, min: &FSEValue, max: &FSEValue) -> bool {
    match (value, min, max) {
        (FSEValue::Integer(value), FSEValue::Integer(min), FSEValue::Integer(max)) => {
            min <= value && value <= max
        }
        (FSEValue::Float(value), FSEValue::Float(min), FSEValue::Float(max)) => {
            min <= value && value <= max
        }
        (
            FSEValue::TimestampMillis(value),
            FSEValue::TimestampMillis(min),
            FSEValue::TimestampMillis(max),
        ) => min <= value && value <= max,
        _ => false,
    }
}
