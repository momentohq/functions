//! Host interfaces for working with AWS DynamoDB
//!
//! It is recommended that you write type bindings for the types in your tables.
//! See the examples on [Item] for how to do this.

use std::collections::HashMap;

use base64::Engine;
use momento_functions_wit::host::momento::host;
use serde::{Deserialize, Serialize};

use crate::FunctionResult;

use super::auth;

/// Dynamodb client for host interfaces.
///
/// This client uses Momento's host-provided AWS communication channel, which
/// is kept hot at all times. When your Function has not run in several days or more,
/// the channel is still hot and ready, keeping your Function invocations predictable
/// even when your demand is unpredictable.
pub struct DynamoDBClient {
    client: host::aws_ddb::Client,
}

impl DynamoDBClient {
    /// Create a new DynamoDB client.
    ///
    /// ```rust
    /// # use momento_functions_host::aws::auth::AwsCredentialsProvider;
    /// # use momento_functions_host::aws::ddb::DynamoDBClient;
    /// # use momento_functions_host::build_environment_aws_credentials;
    /// # use momento_functions_host::FunctionResult;
    /// # fn f() -> FunctionResult<()> {
    /// let client = DynamoDBClient::new(
    ///     &AwsCredentialsProvider::new(
    ///         "us-east-1",
    ///         build_environment_aws_credentials!()
    ///     )?
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(credentials: &auth::AwsCredentialsProvider) -> Self {
        Self {
            client: host::aws_ddb::Client::new(credentials.resource()),
        }
    }

    /// Get an item from a DynamoDB table.
    ///
    /// Examples:
    /// ________
    /// Custom bound types:
    /// ```rust
    /// use momento_functions_host::aws::ddb::{AttributeValue, DynamoDBClient, Item};
    /// use momento_functions_host::{FunctionResult, Error};
    ///
    /// /// Look up an item from a DynamoDB table and deserialize it into a MyStruct.
    /// /// Returns None if the item does not exist.
    /// fn get_my_struct(client: &DynamoDBClient, which_one: &str) -> FunctionResult<Option<MyStruct>> {
    ///     client.get_item("my_table", ("some_attribute", which_one))
    /// }
    ///
    /// struct MyStruct {
    ///     some_attribute: String,
    /// }
    ///
    /// // Boilerplate to convert into dynamodb format
    /// impl From<MyStruct> for Item {
    ///     fn from(value: MyStruct) -> Self {
    ///         [
    ///             ("some_attribute", AttributeValue::from(value.some_attribute)),
    ///         ].into()
    ///     }
    /// }
    ///
    /// // Boilerplate to convert from dynamodb format
    /// impl TryFrom<Item> for MyStruct {
    ///     type Error = Error;
    ///     fn try_from(mut value: Item) -> Result<Self, Self::Error> {
    ///         Ok(Self {
    ///             some_attribute: value.attributes.remove("some_attribute").ok_or("missing some_attribute")?.try_into()?,
    ///         })
    ///     }
    /// }
    pub fn get_item<V, E>(
        &self,
        table_name: impl Into<String>,
        key: impl Into<Key>,
    ) -> FunctionResult<Option<V>>
    where
        V: TryFrom<Item, Error = E>,
        crate::Error: From<E>,
    {
        match self.get_item_raw(table_name, key)? {
            Some(item) => Ok(Some(V::try_from(item)?)),
            None => Ok(None),
        }
    }

    /// Get an item from a DynamoDB table.
    ///
    /// Examples:
    /// ________
    /// ```rust
    /// use momento_functions_host::aws::ddb::{DynamoDBClient, Item};
    /// use momento_functions_host::FunctionResult;
    ///
    /// /// Read an item from a DynamoDB table "my_table" with a S key attribute "some_attribute".
    /// fn get_some_item(client: &DynamoDBClient, which_one: &str) -> FunctionResult<Option<Item>> {
    ///     client.get_item_raw("my_table", ("some_attribute", which_one))
    /// }
    /// ```
    pub fn get_item_raw(
        &self,
        table_name: impl Into<String>,
        key: impl Into<Key>,
    ) -> FunctionResult<Option<Item>> {
        let key: Key = key.into();

        let output = self.client.get_item(&host::aws_ddb::GetItemRequest {
            table_name: table_name.into(),
            key: key.into(),
            consistent_read: false,
            return_consumed_capacity: host::aws_ddb::ReturnConsumedCapacity::None,
            projection_expression: None,
            expression_attribute_names: None,
        })?;

        match output.item {
            Some(item) => {
                match item {
                    // {
                    //   "profile_picture": { "B": "base64 string" },
                    //   "is_valid": { "BOOL": true },
                    //   "pictures": { "BS": ["base64 1", "base64 2"] },
                    //   "friends": { "L": [{ "S": "bob" }, { "S": "alice" }] },
                    //   "relationship": { "M": { "bob": {"S": "best friend"}, "alice": { "S": "second best friend" } } },
                    //   "age": { "N": "23" },
                    //   "favorite_birthdays": { "NS": ["17", "25"] },
                    //   "children": { "NULL": true },
                    //   "name": { "S": "arthur" },
                    //   "friends": { "SS": ["bob", "alice"] }
                    // }
                    host::aws_ddb::Item::Json(j) => serde_json::from_str(&j).map_err(|e| {
                        crate::Error::MessageError(format!(
                            "failed to deserialize host json as item: {e}"
                        ))
                    }),
                }
            }
            None => Ok(None),
        }
    }

    /// Put an item into a DynamoDB table.
    ///
    /// Examples:
    /// Raw item:
    /// ________
    /// ```rust
    /// # use momento_functions_host::aws::ddb::DynamoDBClient;
    /// # use momento_functions_host::FunctionResult;
    ///
    /// # fn put_some_item(client: &DynamoDBClient) -> FunctionResult<()> {
    /// client.put_item(
    ///     "my_table",
    ///     [
    ///         ("some_attribute", "some S value"),
    ///         ("some_other_attribute", "some other S value"),
    ///     ]
    /// )
    /// # }
    /// ```
    /// ________
    /// Custom bound types:
    /// ```rust
    /// use momento_functions_host::aws::ddb::{AttributeValue, DynamoDBClient, Item};
    /// use momento_functions_host::{FunctionResult, Error};
    ///
    /// /// Store an item in a DynamoDB table by serializing a MyStruct.
    /// fn put_my_struct(client: &DynamoDBClient, which_one: MyStruct) -> FunctionResult<()> {
    ///     client.put_item("my_table", which_one)
    /// }
    ///
    /// struct MyStruct {
    ///     some_attribute: String,
    /// }
    ///
    /// // Boilerplate to convert into dynamodb format
    /// impl From<MyStruct> for Item {
    ///     fn from(value: MyStruct) -> Self {
    ///         [
    ///             ("some_attribute", AttributeValue::from(value.some_attribute)),
    ///         ].into()
    ///     }
    /// }
    ///
    /// // Boilerplate to convert from dynamodb format
    /// impl TryFrom<Item> for MyStruct {
    ///     type Error = Error;
    ///     fn try_from(mut value: Item) -> Result<Self, Self::Error> {
    ///         Ok(Self {
    ///             some_attribute: value.attributes.remove("some_attribute").ok_or("missing some_attribute")?.try_into()?,
    ///         })
    ///     }
    /// }
    pub fn put_item(
        &self,
        table_name: impl Into<String>,
        item: impl Into<Item>,
    ) -> FunctionResult<()> {
        let item: Item = item.into();

        let _output = self.client.put_item(&host::aws_ddb::PutItemRequest {
            table_name: table_name.into(),
            item: host::aws_ddb::Item::Json(serde_json::to_string(&item).map_err(|e| {
                crate::Error::MessageError(format!("failed to serialize item as json: {e}"))
            })?),
            condition: None,
            return_values: host::aws_ddb::ReturnValues::None,
            return_consumed_capacity: host::aws_ddb::ReturnConsumedCapacity::None,
        })?;

        Ok(())
    }
}

/// DynamoDB key type
pub enum Key {
    /// Hash key only
    Hash {
        /// Hash key name
        key: String,
        /// Hash key value
        value: KeyValue,
    },
    /// Hash and range key
    HashRange {
        /// Hash key name
        hash_key: String,
        /// Hash key value
        hash_value: KeyValue,
        /// Range key name
        range_key: String,
        /// Range key value
        range_value: KeyValue,
    },
}

/// DynamoDB value type for keys
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum KeyValue {
    /// S value
    #[serde(rename = "S")]
    String(String),
    /// N value
    #[serde(rename = "N")]
    Number(i64),
    /// B value
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

impl From<Key> for Vec<host::aws_ddb::KeyAttribute> {
    fn from(value: Key) -> Self {
        match value {
            Key::Hash { key, value } => vec![host::aws_ddb::KeyAttribute {
                name: key,
                value: value.into(),
            }],
            Key::HashRange {
                hash_key,
                hash_value,
                range_key,
                range_value,
            } => vec![
                host::aws_ddb::KeyAttribute {
                    name: hash_key,
                    value: hash_value.into(),
                },
                host::aws_ddb::KeyAttribute {
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
impl From<KeyValue> for host::aws_ddb::KeyValue {
    fn from(value: KeyValue) -> Self {
        match value {
            KeyValue::String(s) => host::aws_ddb::KeyValue::S(s),
            KeyValue::Number(n) => host::aws_ddb::KeyValue::N(n.to_string()),
            KeyValue::Binary(b) => host::aws_ddb::KeyValue::B(
                base64::engine::general_purpose::STANDARD_NO_PAD.encode(b),
            ),
        }
    }
}

impl From<host::aws_ddb::DdbError> for crate::Error {
    fn from(e: host::aws_ddb::DdbError) -> Self {
        match e {
            host::aws_ddb::DdbError::Unauthorized(u) => Self::MessageError(u),
            host::aws_ddb::DdbError::Malformed(s) => Self::MessageError(s),
            host::aws_ddb::DdbError::Other(o) => Self::MessageError(o),
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
/// This stuff exists mostly because WIT maintainers consider list<t> to be dependent on t,
/// which causes much consternation with regard to serialization. Eventually they will
/// likely work it out like json, protocol buffers, msgpack, and many other serialization
/// formats before it.
///
/// Examples:
/// ________
/// Basic explicit lists:
/// ```rust
/// use momento_functions_host::aws::ddb::Item;
/// let item: Item = vec![("some key", "some value")].into();
/// let item: Item = vec![("some key", 42)].into();
/// ```
/// ________
/// Custom bound types:
/// ```rust
/// use momento_functions_host::aws::ddb::{AttributeValue, Item};
/// use momento_functions_host::Error;
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
///     type Error = Error;
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
    /// The item object
    #[serde(flatten)]
    pub attributes: HashMap<String, AttributeValue>,
}

/// A value within the item object
#[derive(Debug, Serialize, Deserialize)]
pub enum AttributeValue {
    /// A B value
    #[serde(rename = "B")]
    Binary(String),
    /// A BOOL value
    #[serde(rename = "BOOL")]
    Boolean(bool),
    /// A BS value
    #[serde(rename = "BS")]
    BinarySet(Vec<String>),
    /// An L value
    #[serde(rename = "L")]
    List(Vec<AttributeValue>),
    /// An M value
    #[serde(rename = "M")]
    Map(HashMap<String, AttributeValue>),
    /// An N value
    #[serde(rename = "N")]
    Number(String),
    /// An NS value
    #[serde(rename = "NS")]
    NumberSet(Vec<String>),
    /// A NULL value
    #[serde(rename = "NULL")]
    Null(bool),
    /// An S value
    #[serde(rename = "S")]
    String(String),
    /// An SS value
    #[serde(rename = "SS")]
    StringSet(Vec<String>),
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
    type Error = crate::Error;
    fn try_from(value: AttributeValue) -> FunctionResult<Self> {
        match value {
            AttributeValue::String(s) => Ok(s),
            _ => Err(Self::Error::MessageError("not a string".to_string())),
        }
    }
}
impl TryFrom<AttributeValue> for bool {
    type Error = crate::Error;
    fn try_from(value: AttributeValue) -> FunctionResult<Self> {
        match value {
            AttributeValue::Boolean(b) => Ok(b),
            _ => Err(Self::Error::MessageError("not a bool".to_string())),
        }
    }
}
impl TryFrom<AttributeValue> for i64 {
    type Error = crate::Error;
    fn try_from(value: AttributeValue) -> FunctionResult<Self> {
        match value {
            AttributeValue::Number(n) => n
                .parse::<i64>()
                .map_err(|e| Self::Error::MessageError(format!("invalid number: {e}"))),
            _ => Err(Self::Error::MessageError("not a number".to_string())),
        }
    }
}
impl TryFrom<AttributeValue> for Vec<u8> {
    type Error = crate::Error;
    fn try_from(value: AttributeValue) -> FunctionResult<Self> {
        match value {
            AttributeValue::Binary(b) => base64::engine::general_purpose::STANDARD_NO_PAD
                .decode(b)
                .map_err(|e| Self::Error::MessageError(format!("invalid base64: {e}"))),
            _ => Err(Self::Error::MessageError("not a binary".to_string())),
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
