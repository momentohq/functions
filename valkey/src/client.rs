use momento_functions_bytes::Data;
use thiserror::Error;

use crate::{
    command::Command,
    wit::momento::valkey::valkey::{
        self, ClusterClient as WitClusterClient, Command as WitCommand,
        ResponseStream as WitResponseStream, Value as WitValue,
    },
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

/// A lazy stream of [`Value`]s from a bulk Valkey response.
///
/// Implements [`Iterator`] so values are pulled from the host one at a time.
/// Collect into a `Vec` if you need all values at once:
///
/// ```rust,no_run
/// use momento_functions_valkey::{get_managed_cluster_client, Command, Value};
///
/// let client = get_managed_cluster_client("my-cluster");
/// match client.command(Command::get("my_key")) {
///     Ok(Value::Bulk(bulk)) => {
///         let all: Vec<Value> = bulk.collect();
///     }
///     Ok(_) => {}
///     Err(e) => log::error!("command failed: {e}"),
/// }
/// ```
pub struct Bulk {
    stream: WitResponseStream,
}

impl Bulk {
    fn new(stream: WitResponseStream) -> Self {
        Self { stream }
    }
}

impl Iterator for Bulk {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        self.stream.next().map(wit_to_valkey_value)
    }
}

/// A response value from a Valkey command.
pub enum Value {
    /// A nil response from the server.
    Nil,
    /// An integer response.
    Int(i64),
    /// Arbitrary binary data.
    Data(Data),
    /// A bulk response of nested values. Implements [`Iterator`] so values
    /// are streamed from the host rather than loaded all at once.
    Bulk(Bulk),
    /// An OK response.
    Ok,
    /// A short, non-binary string.
    SimpleString(String),
    /// A short, non-binary error string.
    SimpleError(String),
}

fn wit_to_valkey_value(value: WitValue) -> Value {
    match value {
        WitValue::Nil => Value::Nil,
        WitValue::Int(i) => Value::Int(i),
        WitValue::Data(d) => Value::Data(Data::from(d)),
        WitValue::Ok => Value::Ok,
        WitValue::SimpleString(s) => Value::SimpleString(s),
        WitValue::SimpleError(s) => Value::SimpleError(s),
        WitValue::Bulk(stream) => Value::Bulk(Bulk::new(stream)),
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
    /// Accepts a [`Command`] (via convenience constructors or [`Command::builder`]),
    /// or a bare `&str` / `String` for no-argument commands.
    ///
    /// # Examples
    /// ________
    /// Using a convenience constructor:
    /// ```rust,no_run
    /// use momento_functions_valkey::{get_managed_cluster_client, Command};
    ///
    /// let client = get_managed_cluster_client("my-cluster");
    /// match client.command(Command::set("my_key", "my_value")) {
    ///     Ok(_) => {}
    ///     Err(e) => log::error!("command failed: {e}"),
    /// }
    /// ```
    /// ________
    /// Using the builder for custom commands:
    /// ```rust,no_run
    /// use momento_functions_valkey::{get_managed_cluster_client, Command};
    ///
    /// let client = get_managed_cluster_client("my-cluster");
    /// let mut cmd = Command::builder("ZADD");
    /// cmd.argument("my_sorted_set").argument("1.0").argument("member");
    /// match client.command(cmd) {
    ///     Ok(_) => {}
    ///     Err(e) => log::error!("command failed: {e}"),
    /// }
    /// ```
    pub fn command(&self, command: impl Into<Command>) -> Result<Value, ValkeyError> {
        let cmd: Command = command.into();
        let wit_command = WitCommand {
            command: cmd.command,
            arguments: cmd.arguments.into_iter().map(|d| d.into()).collect(),
        };
        self.inner
            .command(wit_command)
            .map(wit_to_valkey_value)
            .map_err(Into::into)
    }
}

/// Get a client to the managed Valkey cluster with the given name.
///
/// # Examples
/// ________
/// Execute a ping using a bare string:
/// ```rust,no_run
/// use momento_functions_valkey::{get_managed_cluster_client, Value};
///
/// let client = get_managed_cluster_client("my-cluster");
/// match client.command("PING") {
///     Ok(Value::SimpleString(s)) => println!("Received: {s}"),
///     Ok(_) => println!("Unexpected response"),
///     Err(e) => log::error!("ping failed: {e}"),
/// }
/// ```
pub fn get_managed_cluster_client(cluster_name: impl Into<String>) -> ClusterClient {
    let inner = valkey::get_managed_cluster_client(&cluster_name.into());
    ClusterClient::new(inner)
}
