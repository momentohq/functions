//! DynamoDB types for items, keys, and attribute values.
//!
//! It is recommended that you write type bindings for the types in your tables.
//! See the examples on [`Item`] for how to do this.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::wit::momento::aws_ddb::aws_ddb;

/// DynamoDB key type.
pub enum Key {
    /// Hash key only.
    Hash {
        /// Hash key name.
        key: String,
        /// Hash key value.
        value: KeyValue,
    },
    /// Hash and range key.
    HashRange {
        /// Hash key name.
        hash_key: String,
        /// Hash key value.
        hash_value: KeyValue,
        /// Range key name.
        range_key: String,
        /// Range key value.
        range_value: KeyValue,
    },
}

/// DynamoDB value type for keys.
#[derive(Debug, Serialize, Deserialize)]
pub enum KeyValue {
    /// S value.
    #[serde(rename = "S")]
    String(String),
    /// N value.
    #[serde(rename = "N")]
    Number(i64),
    /// B value.
    #[serde(rename = "B")]
    Binary(Vec<u8>),
}

impl<K, V> From<(K, V)> for Key
where
    K: Into<String>,
    V: Into<KeyValue>,
{
    fn from((k, v): (K, V)) -> Self {
        Key::Hash {
            key: k.into(),
            value: v.into(),
        }
    }
}

impl From<Key> for Vec<aws_ddb::KeyAttribute> {
    fn from(value: Key) -> Self {
        match value {
            Key::Hash { key, value } => vec![aws_ddb::KeyAttribute {
                name: key,
                value: value.into(),
            }],
            Key::HashRange {
                hash_key,
                hash_value,
                range_key,
                range_value,
            } => vec![
                aws_ddb::KeyAttribute {
                    name: hash_key,
                    value: hash_value.into(),
                },
                aws_ddb::KeyAttribute {
                    name: range_key,
                    value: range_value.into(),
                },
            ],
        }
    }
}

impl From<String> for KeyValue {
    fn from(value: String) -> Self {
        KeyValue::String(value)
    }
}
impl From<&str> for KeyValue {
    fn from(value: &str) -> Self {
        KeyValue::String(value.to_string())
    }
}
impl From<i64> for KeyValue {
    fn from(value: i64) -> Self {
        KeyValue::Number(value)
    }
}
impl From<Vec<u8>> for KeyValue {
    fn from(value: Vec<u8>) -> Self {
        KeyValue::Binary(value)
    }
}
impl From<&[u8]> for KeyValue {
    fn from(value: &[u8]) -> Self {
        KeyValue::Binary(value.to_vec())
    }
}
impl From<KeyValue> for aws_ddb::KeyValue {
    fn from(value: KeyValue) -> Self {
        match value {
            KeyValue::String(s) => aws_ddb::KeyValue::S(s),
            KeyValue::Number(n) => aws_ddb::KeyValue::N(n.to_string()),
            KeyValue::Binary(b) => {
                aws_ddb::KeyValue::B(base64::engine::general_purpose::STANDARD_NO_PAD.encode(b))
            }
        }
    }
}

/// dynamodb-formatted json looks something like this:
/// ```json
/// {
///   "profile_picture": { "B": "base64 string" },
///   "is_valid": { "BOOL": true },
///   "pictures": { "BS": ["base64 1", "base64 2"] },
///   "friends": { "L": [{ "S": "bob" }, { "S": "alice" }] },
///   "relationship": { "M": { "bob": {"S": "best friend"}, "alice": { "S": "second best friend" } } },
///   "age": { "N": "23" },
///   "favorite_birthdays": { "NS": ["17", "25"] },
///   "children": { "NULL": true },
///   "name": { "S": "arthur" },
///   "friends": { "SS": ["bob", "alice"] }
/// }
/// ```
///
/// Examples:
/// ________
/// Basic explicit lists:
/// ```rust
/// use momento_functions_aws_ddb::Item;
/// let item: Item = vec![("some key", "some value")].into();
/// let item: Item = vec![("some key", 42)].into();
/// ```
/// ________
/// Custom bound types:
/// ```rust
/// use momento_functions_aws_ddb::{AttributeValue, Item};
/// struct MyStruct {
///     some_attribute: String,
/// }
///
/// // convert into dynamodb format
/// impl From<MyStruct> for Item {
///     fn from(value: MyStruct) -> Self {
///         [
///             ("some_attribute", AttributeValue::from(value.some_attribute)),
///         ].into()
///     }
/// }
///
/// // convert from dynamodb format
/// impl TryFrom<Item> for MyStruct {
///     type Error = String;
///     fn try_from(mut value: Item) -> Result<Self, Self::Error> {
///         Ok(Self {
///             some_attribute: value.attributes.remove("some_attribute").ok_or("missing some_attribute")?.try_into()?,
///         })
///     }
/// }
///
/// let item: Item = MyStruct { some_attribute: "some value".to_string() }.into();
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct Item {
    /// The item object.
    #[serde(flatten)]
    pub attributes: HashMap<String, AttributeValue>,
}

/// A value within the item object.
#[derive(Debug, Serialize, Deserialize)]
pub enum AttributeValue {
    /// A B value.
    #[serde(rename = "B")]
    Binary(String),
    /// A BOOL value.
    #[serde(rename = "BOOL")]
    Boolean(bool),
    /// A BS value.
    #[serde(rename = "BS")]
    BinarySet(Vec<String>),
    /// An L value.
    #[serde(rename = "L")]
    List(Vec<AttributeValue>),
    /// An M value.
    #[serde(rename = "M")]
    Map(HashMap<String, AttributeValue>),
    /// An N value.
    #[serde(rename = "N")]
    Number(String),
    /// An NS value.
    #[serde(rename = "NS")]
    NumberSet(Vec<String>),
    /// A NULL value.
    #[serde(rename = "NULL")]
    Null(bool),
    /// An S value.
    #[serde(rename = "S")]
    String(String),
    /// An SS value.
    #[serde(rename = "SS")]
    StringSet(Vec<String>),
}

impl AttributeValue {
    fn type_name(&self) -> String {
        match self {
            AttributeValue::Binary(_) => "Binary".to_string(),
            AttributeValue::Boolean(_) => "Boolean".to_string(),
            AttributeValue::BinarySet(_) => "BinarySet".to_string(),
            AttributeValue::List(_) => "List".to_string(),
            AttributeValue::Map(_) => "Map".to_string(),
            AttributeValue::Number(_) => "Number".to_string(),
            AttributeValue::NumberSet(_) => "NumberSet".to_string(),
            AttributeValue::Null(_) => "Null".to_string(),
            AttributeValue::String(_) => "String".to_string(),
            AttributeValue::StringSet(_) => "StringSet".to_string(),
        }
    }
}

/// An error occurred while converting from an AttributeValue.
#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    /// The AttributeValue was not of the expected type.
    #[error("Attribute was not of expected type. Expected: {expected}, Actual: {actual}")]
    WrongType {
        /// The expected AttributeValue type.
        expected: String,
        /// The actual AttributeValue type.
        actual: String,
    },
}

/// An error occurred while converting from an AttributeValue to a numeric type.
#[derive(Debug, thiserror::Error)]
pub enum NumericConversionError {
    /// The AttributeValue was not of the expected type.
    #[error("Attribute was not of expected type. Expected: {expected}, Actual: {actual}")]
    WrongType {
        /// The expected AttributeValue type.
        expected: String,
        /// The actual AttributeValue type.
        actual: String,
    },
    /// Failed to parse an integer value.
    #[error("ParseInt error: {cause}")]
    ParseInt {
        /// The underlying parse error.
        #[from]
        cause: std::num::ParseIntError,
    },
}

/// An error occurred while converting from an AttributeValue to Bytes.
#[derive(Debug, thiserror::Error)]
pub enum BinaryConversionError {
    /// The AttributeValue was not of the expected type.
    #[error("Attribute was not of expected type. Expected: {expected}, Actual: {actual}")]
    WrongType {
        /// The expected AttributeValue type.
        expected: String,
        /// The actual AttributeValue type.
        actual: String,
    },
    /// The AttributeValue did not contain valid base64.
    #[error("Decode error: {cause}")]
    Decode {
        /// The underlying decode error.
        #[from]
        cause: base64::DecodeError,
    },
}

impl From<String> for AttributeValue {
    fn from(value: String) -> Self {
        AttributeValue::String(value)
    }
}
impl From<&str> for AttributeValue {
    fn from(value: &str) -> Self {
        AttributeValue::String(value.to_string())
    }
}
impl From<bool> for AttributeValue {
    fn from(value: bool) -> Self {
        AttributeValue::Boolean(value)
    }
}
impl From<Vec<AttributeValue>> for AttributeValue {
    fn from(value: Vec<AttributeValue>) -> Self {
        AttributeValue::List(value)
    }
}
impl<S> From<HashMap<S, AttributeValue>> for AttributeValue
where
    S: Into<String>,
{
    fn from(value: HashMap<S, AttributeValue>) -> Self {
        AttributeValue::Map(value.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }
}
impl From<i64> for AttributeValue {
    fn from(value: i64) -> Self {
        AttributeValue::Number(value.to_string())
    }
}
impl From<Vec<u8>> for AttributeValue {
    fn from(value: Vec<u8>) -> Self {
        AttributeValue::Binary(base64::engine::general_purpose::STANDARD_NO_PAD.encode(value))
    }
}

impl TryFrom<AttributeValue> for String {
    type Error = ConversionError;
    fn try_from(value: AttributeValue) -> Result<Self, Self::Error> {
        match value {
            AttributeValue::String(s) => Ok(s),
            _ => Err(ConversionError::WrongType {
                actual: value.type_name(),
                expected: "String".to_string(),
            }),
        }
    }
}
impl TryFrom<AttributeValue> for bool {
    type Error = ConversionError;
    fn try_from(value: AttributeValue) -> Result<Self, Self::Error> {
        match value {
            AttributeValue::Boolean(b) => Ok(b),
            _ => Err(ConversionError::WrongType {
                actual: value.type_name(),
                expected: "Boolean".to_string(),
            }),
        }
    }
}
impl TryFrom<AttributeValue> for i64 {
    type Error = NumericConversionError;
    fn try_from(value: AttributeValue) -> Result<Self, Self::Error> {
        match value {
            AttributeValue::Number(n) => n.parse::<i64>().map_err(NumericConversionError::from),
            _ => Err(NumericConversionError::WrongType {
                actual: value.type_name(),
                expected: "Number".to_string(),
            }),
        }
    }
}
impl TryFrom<AttributeValue> for Vec<u8> {
    type Error = BinaryConversionError;
    fn try_from(value: AttributeValue) -> Result<Self, Self::Error> {
        match value {
            AttributeValue::Binary(b) => base64::engine::general_purpose::STANDARD_NO_PAD
                .decode(b)
                .map_err(BinaryConversionError::from),
            _ => Err(BinaryConversionError::WrongType {
                actual: value.type_name(),
                expected: "Binary".to_string(),
            }),
        }
    }
}

impl<I, S, V> From<I> for Item
where
    I: IntoIterator<Item = (S, V)>,
    S: Into<String>,
    V: Into<AttributeValue>,
{
    fn from(value: I) -> Self {
        Item {
            attributes: value
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }
}
