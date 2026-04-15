use momento_functions_bytes::Data;
use thiserror::Error;

use crate::wit::momento::valkey::valkey::{
    self, ClusterClient as WitClusterClient, Command as WitCommand, Value as WitValue,
};

/// An error returned by a Valkey command.
#[derive(Debug, Error)]
pub enum ValkeyError {
    /// The request failed for some other reason.
    #[error("valkey error: {0}")]
    Other(String),
}

impl From<valkey::ValkeyError> for ValkeyError {
    fn from(e: valkey::ValkeyError) -> Self {
        match e {
            valkey::ValkeyError::Other(msg) => ValkeyError::Other(msg),
        }
    }
}

/// A response value from a Valkey command.
pub enum Value {
    /// A nil response from the server.
    Nil,
    /// An integer response.
    Int(i64),
    /// Arbitrary binary data.
    Data(Vec<u8>),
    /// A bulk response of nested values.
    Bulk(Vec<Value>),
    /// An OK response.
    Ok,
    /// A short, non-binary string.
    SimpleString(String),
    /// A short, non-binary error string.
    SimpleError(String),
}

fn resolve_value(value: WitValue) -> Value {
    match value {
        WitValue::Nil => Value::Nil,
        WitValue::Int(i) => Value::Int(i),
        WitValue::Data(d) => Value::Data(Data::from(d).into_bytes()),
        WitValue::Ok => Value::Ok,
        WitValue::SimpleString(s) => Value::SimpleString(s),
        WitValue::SimpleError(s) => Value::SimpleError(s),
        WitValue::Bulk(stream) => {
            let mut items = Vec::new();
            while let Some(v) = stream.next() {
                items.push(resolve_value(v));
            }
            Value::Bulk(items)
        }
    }
}

/// A client for a managed Valkey cluster.
///
/// Obtain a client using [`get_managed_cluster_client`].
pub struct ClusterClient {
    inner: WitClusterClient,
}

impl ClusterClient {
    pub(crate) fn new(inner: WitClusterClient) -> Self {
        Self { inner }
    }

    /// Execute a command against the cluster.
    ///
    /// # Arguments
    /// * `command` - The command name (e.g. `"SET"`, `"GET"`).
    /// * `arguments` - The command arguments.
    ///
    /// # Examples
    /// ________
    /// Set a key:
    /// ```rust,no_run
    /// use momento_functions_valkey::{get_managed_cluster_client, ValkeyError};
    ///
    /// # fn f() -> Result<(), ValkeyError> {
    /// let client = get_managed_cluster_client("my-cluster");
    /// let response = client.command("SET", vec!["my_key".into(), b"my_value".to_vec().into()])?;
    /// # Ok(()) }
    /// ```
    pub fn command(
        &self,
        command: impl Into<String>,
        arguments: Vec<Data>,
    ) -> Result<Value, ValkeyError> {
        let wit_command = WitCommand {
            command: command.into(),
            arguments: arguments.into_iter().map(|d| d.into()).collect(),
        };
        self.inner
            .command(wit_command)
            .map(resolve_value)
            .map_err(Into::into)
    }
}

/// Get a client to the managed Valkey cluster with the given name.
///
/// # Arguments
/// * `cluster_name` - The name of the managed Valkey cluster.
///
/// # Examples
/// ________
/// Get a cluster client and execute a ping:
/// ```rust,no_run
/// use momento_functions_valkey::{get_managed_cluster_client, Value, ValkeyError};
///
/// # fn f() -> Result<(), ValkeyError> {
/// let client = get_managed_cluster_client("my-cluster");
/// let response = client.command("PING", vec![])?;
/// match response {
///     Value::SimpleString(s) => println!("Received: {s}"),
///     _ => println!("Unexpected response"),
/// }
/// # Ok(()) }
/// ```
pub fn get_managed_cluster_client(cluster_name: impl Into<String>) -> ClusterClient {
    let inner = valkey::get_managed_cluster_client(&cluster_name.into());
    ClusterClient::new(inner)
}
