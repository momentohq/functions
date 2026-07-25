use crate::types::{AttributeValue, Item, Key};
use crate::wit::momento::aws_ddb::aws_ddb::{self as aws_ddb};
use momento_functions_aws_auth::CredentialsProvider;
use momento_functions_bytes::Data;
use std::collections::HashMap;

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
    Json {
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
    /// ```rust,no_run
    /// use momento_functions_aws_auth::{Authorization, IamRole, provider};
    /// use momento_functions_aws_ddb::DynamoDBClient;
    ///
    /// let credentials = match provider(
    ///     &Authorization::Federated(IamRole { role_arn: "arn:aws:iam::123456789012:role/my-role".to_string() }),
    ///     "us-east-1",
    /// ) {
    ///     Ok(credentials) => credentials,
    ///     Err(e) => {
    ///         eprintln!("failed to build credentials: {e}");
    ///         return;
    ///     }
    /// };
    /// let client = DynamoDBClient::new(&credentials);
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
    /// ```rust,no_run
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
    ///             some_attribute: value.attributes.remove("some_attribute").ok_or("missing some_attribute")?.try_into().map_err(|e: momento_functions_aws_ddb::ConversionError| e.to_string())?,
    ///         })
    ///     }
    /// }
    /// ```
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
    /// ```rust,no_run
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
                aws_ddb::Item::Json(data) => {
                    let bytes = Data::from(data).into_bytes();
                    Ok(serde_json::from_slice(&bytes)?)
                }
            },
            None => Ok(None),
        }
    }

    /// Put an item into a DynamoDB table.
    ///
    /// Examples:
    /// ________
    /// Raw item:
    /// ```rust,no_run
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
    /// ```rust,no_run
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

        let _output = self.client.put_item(aws_ddb::PutItemRequest {
            table_name: table_name.into(),
            item: aws_ddb::Item::Json(Data::from(serde_json::to_vec(&item)?).into()),
            condition: None,
            return_values: aws_ddb::ReturnValues::None,
            return_consumed_capacity: aws_ddb::ReturnConsumedCapacity::None,
        })?;

        Ok(())
    }

    /// Query a table or a secondary index — ONE page of results.
    ///
    /// Unlike [`get_item_raw`](Self::get_item_raw), this is a paginated call: DynamoDB stops at
    /// `limit` or at its 1 MB page cap, whichever comes first. Check
    /// [`QueryPage::last_evaluated_key`] and pass it back via [`Query::start_after`] to continue.
    /// **A page can be empty while more results remain** (everything in it was filtered out), so
    /// looping on "items is empty" instead of on the key silently truncates the result set.
    ///
    /// Examples:
    /// ________
    /// One page:
    /// ```rust,no_run
    /// use momento_functions_aws_ddb::{DynamoDBClient, DynamoDBError, Query};
    ///
    /// # fn recent_for(client: &DynamoDBClient, id: &str) -> Result<(), DynamoDBError> {
    /// let page = client.query(
    ///     Query::new("my_table", "pk = :id")
    ///         .index("my_index")
    ///         .value(":id", id)
    ///         .limit(100),
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    /// ________
    /// Every page — the shape a total (a count, a sum) must use to be correct:
    /// ```rust,no_run
    /// # use momento_functions_aws_ddb::{DynamoDBClient, DynamoDBError, Item, Query};
    /// # fn all_for(client: &DynamoDBClient, id: &str) -> Result<Vec<Item>, DynamoDBError> {
    /// let mut all = Vec::new();
    /// let mut start_key = None;
    /// loop {
    ///     let mut query = Query::new("my_table", "pk = :id").value(":id", id);
    ///     if let Some(key) = start_key {
    ///         query = query.start_after(key);
    ///     }
    ///     let page = client.query(query)?;
    ///     all.extend(page.items);
    ///     start_key = page.last_evaluated_key;
    ///     if start_key.is_none() {
    ///         break;
    ///     }
    /// }
    /// # Ok(all)
    /// # }
    /// ```
    pub fn query(&self, query: Query) -> Result<QueryPage, DynamoDBError> {
        let Query {
            table_name,
            index_name,
            key_condition_expression,
            expression_attribute_values,
            expression_attribute_names,
            filter_expression,
            projection_expression,
            limit,
            scan_index_forward,
            consistent_read,
            exclusive_start_key,
        } = query;

        let expression_attribute_values = match expression_attribute_values {
            Some(attributes) => Some(aws_ddb::Item::Json(
                Data::from(serde_json::to_vec(&Item { attributes })?).into(),
            )),
            None => None,
        };

        let output = self.client.query(aws_ddb::QueryRequest {
            table_name,
            index_name,
            key_condition_expression,
            expression_attribute_values,
            expression_attribute_names,
            filter_expression,
            projection_expression,
            limit,
            scan_index_forward,
            consistent_read,
            exclusive_start_key,
            return_consumed_capacity: aws_ddb::ReturnConsumedCapacity::None,
        })?;

        let mut items = Vec::with_capacity(output.items.len());
        for item in output.items {
            match item {
                aws_ddb::Item::Json(data) => {
                    let bytes = Data::from(data).into_bytes();
                    items.push(serde_json::from_slice(&bytes)?);
                }
            }
        }

        Ok(QueryPage {
            items,
            count: output.count,
            scanned_count: output.scanned_count,
            last_evaluated_key: output.last_evaluated_key,
        })
    }
}

/// One page of a [`DynamoDBClient::query`].
#[derive(Debug)]
pub struct QueryPage {
    /// The matching items, in sort-key order. May be empty while [`Self::last_evaluated_key`] is
    /// `Some` — that page was entirely filtered out, which is NOT the end of the result set.
    pub items: Vec<Item>,
    /// Items returned in this page (after any filter expression).
    pub count: u32,
    /// Items evaluated in this page (before any filter expression).
    pub scanned_count: u32,
    /// `Some` when more pages remain — pass it to [`Query::start_after`]. `None` means complete.
    pub last_evaluated_key: Option<Vec<aws_ddb::KeyAttribute>>,
}

/// A DynamoDB query. Built fluently; `table_name` and the key-condition expression are required
/// because a query without them is not expressible.
#[derive(Debug)]
pub struct Query {
    table_name: String,
    index_name: Option<String>,
    key_condition_expression: String,
    expression_attribute_values: Option<HashMap<String, AttributeValue>>,
    expression_attribute_names: Option<Vec<(String, String)>>,
    filter_expression: Option<String>,
    projection_expression: Option<String>,
    limit: Option<u32>,
    scan_index_forward: Option<bool>,
    consistent_read: bool,
    exclusive_start_key: Option<Vec<aws_ddb::KeyAttribute>>,
}

impl Query {
    /// A query over `table_name` constrained by `key_condition_expression` (e.g. `"pk = :id"`).
    /// Bind every `:placeholder` it references with [`value`](Self::value).
    pub fn new(table_name: impl Into<String>, key_condition_expression: impl Into<String>) -> Self {
        Query {
            table_name: table_name.into(),
            index_name: None,
            key_condition_expression: key_condition_expression.into(),
            expression_attribute_values: None,
            expression_attribute_names: None,
            filter_expression: None,
            projection_expression: None,
            limit: None,
            scan_index_forward: None,
            consistent_read: false,
            exclusive_start_key: None,
        }
    }

    /// Query a secondary index instead of the base table.
    pub fn index(mut self, index_name: impl Into<String>) -> Self {
        self.index_name = Some(index_name.into());
        self
    }

    /// Bind one `:placeholder` used by the key condition or filter, e.g.
    /// `.value(":id", ("S", "abc"))`.
    pub fn value(
        mut self,
        placeholder: impl Into<String>,
        value: impl Into<AttributeValue>,
    ) -> Self {
        self.expression_attribute_values
            .get_or_insert_with(HashMap::new)
            .insert(placeholder.into(), value.into());
        self
    }

    /// Alias a reserved word used in an expression, e.g. `.name("#n", "name")`.
    pub fn name(mut self, placeholder: impl Into<String>, attribute: impl Into<String>) -> Self {
        self.expression_attribute_names
            .get_or_insert_with(Vec::new)
            .push((placeholder.into(), attribute.into()));
        self
    }

    /// Filter results server-side AFTER the key condition. Does not reduce read cost — see the
    /// `filter-expression` note on the host interface.
    pub fn filter(mut self, filter_expression: impl Into<String>) -> Self {
        self.filter_expression = Some(filter_expression.into());
        self
    }

    /// Return only these attributes.
    pub fn project(mut self, projection_expression: impl Into<String>) -> Self {
        self.projection_expression = Some(projection_expression.into());
        self
    }

    /// Cap items EVALUATED per page (before the filter), not items returned.
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Walk the sort key in descending order (newest-first, for a timestamp sort key).
    pub fn descending(mut self) -> Self {
        self.scan_index_forward = Some(false);
        self
    }

    /// Read strongly-consistently. Invalid against a global secondary index.
    pub fn consistent_read(mut self) -> Self {
        self.consistent_read = true;
        self
    }

    /// Continue from a previous page's [`QueryPage::last_evaluated_key`].
    pub fn start_after(mut self, exclusive_start_key: Vec<aws_ddb::KeyAttribute>) -> Self {
        self.exclusive_start_key = Some(exclusive_start_key);
        self
    }
}
