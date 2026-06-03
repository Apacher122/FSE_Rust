//! FSE-native scalar values.

/// Scalar logical value stored before semantic encoding.
///
/// # Runtime Role
///
/// `FSEValue` represents typed source data before it is mapped into numeric
/// coordinate space by an encoder.
#[derive(Clone, Debug, PartialEq)]
pub enum FSEValue {
    /// Signed 64-bit integer value.
    Integer(i64),

    /// 64-bit floating point value.
    Float(f64),

    /// UTF-8 string value.
    Text(String),

    /// Boolean value.
    Boolean(bool),

    /// Timestamp value represented as milliseconds since the Unix epoch.
    TimestampMillis(i64),

    /// Categorical value represented by a stable label.
    Category(String),

    /// Missing value.
    Null,
}

impl FSEValue {
    /// Returns the non-null field type for this value.
    pub fn field_type(&self) -> Option<FSEFieldType> {
        match self {
            Self::Integer(_) => Some(FSEFieldType::Integer),
            Self::Float(_) => Some(FSEFieldType::Float),
            Self::Text(_) => Some(FSEFieldType::Text),
            Self::Boolean(_) => Some(FSEFieldType::Boolean),
            Self::TimestampMillis(_) => Some(FSEFieldType::TimestampMillis),
            Self::Category(_) => Some(FSEFieldType::Category),
            Self::Null => None,
        }
    }

    /// Returns true when this value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

/// Logical field type for FSE-native data.
///
/// # Runtime Role
///
/// Field types describe the input semantics that later encoders must preserve
/// when mapping records into coordinate space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FSEFieldType {
    /// Signed 64-bit integer field.
    Integer,

    /// 64-bit floating point field.
    Float,

    /// UTF-8 string field.
    Text,

    /// Boolean field.
    Boolean,

    /// Timestamp field represented as milliseconds since the Unix epoch.
    TimestampMillis,

    /// Categorical field represented by stable labels.
    Category,
}
