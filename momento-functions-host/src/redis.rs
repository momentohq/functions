//! Host interfaces for working with redis or valkey

use momento_functions_wit::host::momento::host;

use crate::{
    FunctionResult,
    encoding::{Encode, Extract},
};

/// Redis client for Function host interfaces.
///
/// This client is used to connect to a Redis or Valkey instance that you own.
///
/// This client uses Momento's host-provided connection cache, which keeps connections
/// alive across invocations of your Function for reuse.
pub struct RedisClient {
    client: host::redis::Client,
}

impl RedisClient {
    /// Create a new Redis client from a connection string.
    ///
    /// Note that the redis/valkey you are connecting to must be accessible to the
    /// Functions host environment. If you are using public Momento endpoints, you
    /// will only be able to connect to public caches - that is not a reasonable
    /// way to set up a production environment. If you want to use a private cache
    /// for a real application, please get in touch with support@momentohq.com
    ///
    /// ```rust
    /// # use momento_functions_host::redis::RedisClient;
    /// # fn f() -> FunctionResult<()> {
    /// let client = RedisClient::new("valkey://my.valkey.instance:6379");
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(connection_string: impl Into<String>) -> Self {
        Self {
            client: host::redis::Client::new(&host::redis::RedisConnectionType::BasicConnection(
                connection_string.into(),
            )),
        }
    }

    /// Get a value from Redis by key.
    pub fn get<T: Extract>(&self, key: impl Into<Vec<u8>>) -> FunctionResult<Option<T>> {
        let response = self.client.pipe(&[host::redis::Command {
            command: "get".to_string(),
            arguments: vec![key.into()],
        }])?;
        Ok(match response.next() {
            Some(value) => {
                log::debug!("Redis get response: {value:?}");
                match value {
                    host::redis::Value::Nil => None,
                    host::redis::Value::Int(i) => Some(T::extract(i.to_string().into_bytes())?),
                    host::redis::Value::Data(value) => Some(T::extract(value)?),
                    host::redis::Value::Bulk(response_stream) => {
                        return Err(crate::Error::MessageError(format!(
                            "Bulk response not supported in this context {response_stream:?}"
                        )));
                    }
                    host::redis::Value::Status(status) => {
                        return Err(crate::Error::MessageError(status));
                    }
                    host::redis::Value::Okay => {
                        return Err(crate::Error::MessageError(
                            "Okay response not supported in this context".into(),
                        ));
                    }
                }
            }
            None => None,
        })
    }

    /// Set a value in Redis with a key.
    pub fn set<T: Encode>(&self, key: impl Into<Vec<u8>>, value: T) -> FunctionResult<()> {
        let serialized_value = value.try_serialize()?.into();
        let response = self.client.pipe(&[host::redis::Command {
            command: "set".to_string(),
            arguments: vec![key.into(), serialized_value],
        }])?;
        match response.next() {
            Some(host::redis::Value::Okay) => Ok(()),
            Some(host::redis::Value::Status(status)) => Err(crate::Error::MessageError(status)),
            e => Err(crate::Error::MessageError(format!(
                "unexpected response: {e:?}"
            ))),
        }
    }

    /// Delete a key from Redis.
    pub fn delete(&self, key: impl Into<Vec<u8>>) -> FunctionResult<()> {
        let response = self.client.pipe(&[host::redis::Command {
            command: "del".to_string(),
            arguments: vec![key.into()],
        }])?;
        match response.next() {
            Some(host::redis::Value::Int(count)) => {
                log::debug!("delete response: {count}");
                Ok(())
            }
            Some(host::redis::Value::Status(status)) => Err(crate::Error::MessageError(status)),
            e => Err(crate::Error::MessageError(format!(
                "unexpected response: {e:?}"
            ))),
        }
    }

    /// Execute redis commands
    ///
    /// ```rust
    /// # use momento_functions_host::redis::{RedisClient, Command};
    /// # use momento_functions_host::FunctionResult;
    /// # fn f(client: &RedisClient) -> FunctionResult<()> {
    /// let response_stream = client.pipe(vec![
    ///     Command::builder().set("my_key", "my_value")?.build(),
    ///     Command::builder().get("my_key").build(),
    ///     Command::builder()
    ///         .any("FT.SEARCH")
    ///         .arg(r#"test_index "*=>[KNN 5 @vector_a $query_vector]" PARAMS 2 query_vector "\xcd\xccL?\x00\x00\x00\x00\x00\x00\x00\x00""#)
    ///         .build(),
    /// ]);
    ///
    /// #    Ok(())
    /// #}
    /// ```
    pub fn pipe(&self, commands: Vec<Command>) -> FunctionResult<ResponseStream> {
        let response_stream = self.client.pipe(
            &commands
                .into_iter()
                .map(|Command { command, arguments }| host::redis::Command { command, arguments })
                .collect::<Vec<_>>(),
        )?;

        Ok(ResponseStream {
            inner: response_stream,
        })
    }
}

impl From<host::redis::RedisError> for crate::Error {
    fn from(e: host::redis::RedisError) -> Self {
        crate::Error::MessageError(format!("Redis error: {e:?}"))
    }
}

/// A raw redis command
#[derive(Debug, Clone)]
pub struct Command {
    command: String,
    arguments: Vec<Vec<u8>>,
}
impl Command {
    /// A builder for creating redis commands
    pub fn builder() -> CommandBuilder<SelectCommand> {
        CommandBuilder {
            command: SelectCommand,
        }
    }
}

/// A stream of responses from a redis pipe
#[derive(Debug)]
pub struct ResponseStream {
    inner: host::redis::ResponseStream,
}
impl ResponseStream {
    /// Get the next response from the stream
    fn next_value(&mut self) -> Option<RedisValue> {
        let next = self.inner.next();
        next.map(|value| match value {
            host::redis::Value::Nil => RedisValue::Nil,
            host::redis::Value::Int(i) => RedisValue::Int(i),
            host::redis::Value::Data(data) => RedisValue::Data(data),
            host::redis::Value::Bulk(response_stream) => RedisValue::Bulk(ResponseStream {
                inner: response_stream,
            }),
            host::redis::Value::Status(status) => RedisValue::Status(status),
            host::redis::Value::Okay => RedisValue::Okay,
        })
    }
}
impl Iterator for ResponseStream {
    type Item = RedisValue;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_value()
    }
}

/// A value returned from a redis command
#[derive(Debug)]
pub enum RedisValue {
    /// An explicit nil value was returned from the server
    Nil,
    /// An explicit integer value was returned from the server
    Int(i64),
    /// A data blob was returned from the server
    Data(Vec<u8>),
    /// A bulk response stream was returned from the server.
    /// This is used for commands that return multiple values. You iterate over it
    /// to get each individual value.
    Bulk(ResponseStream),
    /// A status message was returned from the server
    Status(String),
    /// An okay response was returned from the server
    Okay,
}
impl RedisValue {
    /// try to extract the value as a specific type
    ///
    /// Only works for Data responses.
    pub fn extract<T: Extract>(self) -> FunctionResult<T> {
        match self {
            RedisValue::Data(data) => T::extract(data),
            v => Err(crate::Error::MessageError(format!(
                "cannot extract value from {v:?}"
            ))),
        }
    }
}

/// A builder for creating raw redis commands
#[derive(Debug, Clone)]
pub struct CommandBuilder<SelectCommand> {
    command: SelectCommand,
}

#[doc(hidden)]
pub struct SelectCommand;
impl CommandBuilder<SelectCommand> {
    /// Set the command to execute
    pub fn get(self, key: impl Into<Vec<u8>>) -> CommandBuilder<Get> {
        CommandBuilder {
            command: Get { key: key.into() },
        }
    }

    /// Set the command to execute
    pub fn set<T: Encode>(
        self,
        key: impl Into<Vec<u8>>,
        value: T,
    ) -> FunctionResult<CommandBuilder<Set>> {
        Ok(CommandBuilder {
            command: Set {
                key: key.into(),
                value: value.try_serialize()?.into(),
                existence_check: Default::default(),
            },
        })
    }

    /// Set the command to execute
    pub fn any(self, command: impl Into<String>) -> CommandBuilder<Any> {
        CommandBuilder {
            command: Any {
                command: command.into(),
                arguments: Default::default(),
            },
        }
    }
}

#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct Get {
    key: Vec<u8>,
}
impl CommandBuilder<Get> {
    /// Finalize the command
    pub fn build(self) -> Command {
        Command {
            command: "get".to_string(),
            arguments: vec![self.command.key],
        }
    }
}

#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct Set {
    key: Vec<u8>,
    value: Vec<u8>,
    existence_check: Option<bool>,
}
impl CommandBuilder<Set> {
    /// Only set the value if the key does not already exist
    pub fn if_not_exists(mut self) -> Self {
        self.command.existence_check = Some(false);
        self
    }

    /// Only set the value if the key already exists
    pub fn if_exists(mut self) -> Self {
        self.command.existence_check = Some(true);
        self
    }

    /// Finalize the command
    pub fn build(self) -> Command {
        let mut arguments = vec![self.command.key, self.command.value];
        if let Some(existence_check) = self.command.existence_check {
            if existence_check {
                arguments.push(b"XX".to_vec());
            } else {
                arguments.push(b"NX".to_vec());
            }
        }
        Command {
            command: "set".to_string(),
            arguments,
        }
    }
}

#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct Any {
    command: String,
    arguments: Vec<Vec<u8>>,
}
impl CommandBuilder<Any> {
    /// Add an argument to the command
    pub fn value<T: Encode>(mut self, arg: T) -> FunctionResult<Self> {
        self.command.arguments.push(arg.try_serialize()?.into());
        Ok(self)
    }

    /// Add a pre-encoded argument to the command
    pub fn arg(mut self, arg: impl Into<Vec<u8>>) -> Self {
        self.command.arguments.push(arg.into());
        self
    }

    /// Finalize the command
    pub fn build(self) -> Command {
        Command {
            command: self.command.command,
            arguments: self.command.arguments,
        }
    }
}
