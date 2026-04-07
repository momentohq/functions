use crate::types::{Item, Key};
use crate::wit::momento::aws_ddb::aws_ddb::{self as aws_ddb};
use momento_functions_aws_auth::CredentialsProvider;

/// DynamoDB client for host interfaces.
///
/// This client uses Momento's host-provided AWS communication channel, which
/// is kept hot at all times. When your Function has not run in several days or more,
/// the channel is still hot and ready, keeping your Function invocations predictable
/// even when your demand is unpredictable.
pub struct DynamoDBClient {
    client: aws_ddb::Client,
}

/// An error returned from a DynamoDB call.
#[derive(Debug, thiserror::Error)]
pub enum DynamoDBError {
    /// When calling DynamoDB, Items are serialized/deserialized to/from JSON.
    /// This error indicates that a failure occurred when doing so.
    #[error("Failed to serialize/deserialize host json: {cause}")]
    SerDeJson {
        /// The underlying (de)serialization error.
        #[from]
        cause: serde_json::error::Error,
    },
    /// An error from the DynamoDB host interface.
    #[error(transparent)]
    Dynamo(#[from] aws_ddb::DdbError),
}

/// An error occurred while using the extracting get_item wrapper.
#[derive(Debug, thiserror::Error)]
pub enum GetItemError<E> {
    /// An error occurred when calling the provided TryFrom implementation.
    TryFrom {
        /// The underlying error.
        cause: E,
    },
    /// An error occurred when calling DynamoDB.
    Dynamo {
        /// The underlying error.
        #[from]
        cause: DynamoDBError,
    },
}

impl DynamoDBClient {
    /// Create a new DynamoDB client.
    ///
    /// ```rust
    /// use momento_functions_aws_auth::{Authorization, IamRole, provider, CredentialsProvider};
    /// use momento_functions_aws_ddb::DynamoDBClient;
    ///
    /// # fn f() -> Result<(), momento_functions_aws_auth::AuthError> {
    /// let credentials = provider(
    ///     Authorization::Federated(IamRole { role_arn: "arn:aws:iam::123456789012:role/my-role".to_string() }),
    ///     "us-east-1",
    /// )?;
    /// let client = DynamoDBClient::new(&credentials);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(credentials: &CredentialsProvider) -> Self {
        Self {
            client: aws_ddb::Client::new(credentials),
        }
    }

    /// Get an item from a DynamoDB table.
    ///
    /// Examples:
    /// ________
    /// Custom bound types:
    /// ```rust
    /// use momento_functions_aws_ddb::{AttributeValue, DynamoDBClient, DynamoDBError, GetItemError, Item};
    ///
    /// /// Look up an item from a DynamoDB table and deserialize it into a MyStruct.
    /// /// Returns None if the item does not exist.
    /// fn get_my_struct(client: &DynamoDBClient, which_one: &str) -> Result<Option<MyStruct>, GetItemError<String>> {
    ///     client.get_item("my_table", ("some_attribute", which_one))
    /// }
    ///
    /// struct MyStruct {
    ///     some_attribute: String,
    /// }
    ///
    /// // Boilerplate to convert from dynamodb format
    ///
    /// impl TryFrom<Item> for MyStruct {
    ///     type Error = String;
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
    ) -> Result<Option<V>, GetItemError<E>>
    where
        V: TryFrom<Item, Error = E>,
    {
        match self.get_item_raw(table_name, key)? {
            Some(item) => Ok(Some(
                V::try_from(item).map_err(|e| GetItemError::TryFrom { cause: e })?,
            )),
            None => Ok(None),
        }
    }

    /// Get an item from a DynamoDB table.
    ///
    /// Examples:
    /// ________
    /// ```rust
    /// use momento_functions_aws_ddb::{DynamoDBClient, DynamoDBError, Item};
    ///
    /// /// Read an item from a DynamoDB table "my_table" with a S key attribute "some_attribute".
    /// fn get_some_item(client: &DynamoDBClient, which_one: &str) -> Result<Option<Item>, DynamoDBError> {
    ///     client.get_item_raw("my_table", ("some_attribute", which_one))
    /// }
    /// ```
    pub fn get_item_raw(
        &self,
        table_name: impl Into<String>,
        key: impl Into<Key>,
    ) -> Result<Option<Item>, DynamoDBError> {
        let key: Key = key.into();

        let output = self.client.get_item(&aws_ddb::GetItemRequest {
            table_name: table_name.into(),
            key: key.into(),
            consistent_read: false,
            return_consumed_capacity: aws_ddb::ReturnConsumedCapacity::None,
            projection_expression: None,
            expression_attribute_names: None,
        })?;

        match output.item {
            Some(item) => match item {
                aws_ddb::Item::Json(j) => Ok(serde_json::from_str(&j)?),
            },
            None => Ok(None),
        }
    }

    /// Put an item into a DynamoDB table.
    ///
    /// Examples:
    /// ________
    /// Raw item:
    /// ```rust
    /// # use momento_functions_aws_ddb::{DynamoDBClient, DynamoDBError};
    ///
    /// # fn put_some_item(client: &DynamoDBClient) -> Result<(), DynamoDBError> {
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
    /// use momento_functions_aws_ddb::{AttributeValue, DynamoDBClient, DynamoDBError, Item};
    ///
    /// /// Store an item in a DynamoDB table by serializing a MyStruct.
    /// fn put_my_struct(client: &DynamoDBClient, which_one: MyStruct) -> Result<(), DynamoDBError> {
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
    /// ```
    pub fn put_item(
        &self,
        table_name: impl Into<String>,
        item: impl Into<Item>,
    ) -> Result<(), DynamoDBError> {
        let item: Item = item.into();

        let _output = self.client.put_item(&aws_ddb::PutItemRequest {
            table_name: table_name.into(),
            item: aws_ddb::Item::Json(serde_json::to_string(&item)?),
            condition: None,
            return_values: aws_ddb::ReturnValues::None,
            return_consumed_capacity: aws_ddb::ReturnConsumedCapacity::None,
        })?;

        Ok(())
    }
}
