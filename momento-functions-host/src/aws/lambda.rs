//! Host interfaces for working with AWS Lambda
use crate::encoding::{Encode, EncodeError, Extract, ExtractError};
use momento_functions_wit::host::momento::host;
use momento_functions_wit::host::momento::host::aws_lambda::LambdaError;

use super::auth;

/// Lambda client for host interfaces.
///
/// This client uses Momento's host-provided AWS communication channel, which
/// is kept hot at all times. When your Function has not run in several days or more,
/// the channel is still hot and ready, keeping your Function invocations predictable
/// even when your demand is unpredictable.
pub struct LambdaClient {
    client: host::aws_lambda::Client,
}

/// An error occurred while invoking a Lambda function.
#[derive(Debug, thiserror::Error)]
pub enum InvokeError<E>
where
    E: EncodeError,
{
    /// An error occurred while encoding the provided payload.
    #[error("Failed to encode payload.")]
    EncodeFailed {
        /// The underlying encode error.
        cause: E,
    },
    /// An error occurred when calling the host invoke function.
    #[error(transparent)]
    LambdaError(#[from] LambdaError),
}

impl LambdaClient {
    /// Create a new Lambda client.
    ///
    /// ```rust
    /// # use momento_functions_host::aws::auth::AwsCredentialsProvider;
    /// # use momento_functions_host::aws::lambda::LambdaClient;
    /// # use momento_functions_host::build_environment_aws_credentials;    /// #
    /// use momento_functions_wit::host::momento::host::aws_auth::AuthError;
    ///
    /// fn f() -> Result<(), AuthError> {
    /// let client = LambdaClient::new(
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
            client: host::aws_lambda::Client::new(credentials.resource()),
        }
    }

    /// Invoke a lambda function.
    ///
    /// You can use strings, bytes, or structs that are Serializable.
    ///
    /// Examples:
    /// ________
    /// ```rust
    /// use momento_functions_host::aws::lambda::{InvokeError, LambdaClient};
    /// use momento_functions_host::encoding::Json;;
    ///
    /// # fn f(client: &LambdaClient) -> Result<(), InvokeError<&str>> {
    /// // With a payload
    /// client.invoke(
    ///     "my_lambda_function",
    ///     "hello world",
    /// )?;
    ///
    /// // With a payload and a qualifier
    /// client.invoke(
    ///     ("my_lambda_function", "v1"),
    ///     "hello world",
    /// )?;
    ///
    /// // Without a payload
    /// client.invoke(
    ///     "my_lambda_function",
    ///     (),
    /// )?;
    ///
    /// // With literal bytes
    /// client.invoke(
    ///     "my_lambda_function",
    ///     b"some literal bytes".to_vec(),
    /// )?;
    /// # Ok(())}
    /// ```
    /// ________
    /// With json-encoded payloads
    /// ```rust
    /// use momento_functions_host::aws::lambda::{InvokeError, LambdaClient};
    /// use momento_functions_host::encoding::Json;
    ///
    /// #[derive(serde::Serialize)]
    /// struct MyStruct {
    ///     hello: String
    /// }
    /// #[derive(serde::Deserialize)]
    /// struct Reply {
    ///     message: String
    /// }
    ///
    /// # fn f(client: &LambdaClient) -> Result<(), InvokeError<Json<MyStruct>>> {
    /// // Just a request payload, encoded as JSON
    /// client.invoke(
    ///     "my_lambda_function",
    ///     Json(MyStruct { hello: "hello".to_string() }),
    /// )?;
    ///
    /// // Request and response payload, both encoded as JSON
    /// let Json(reply): Json<Reply> = client.invoke(
    ///     "my_lambda_function",
    ///     Json(MyStruct { hello: "hello".to_string() }),
    /// )?
    /// .extract()?;
    ///
    /// let message = reply.message;
    /// # Ok(())}
    /// ```
    pub fn invoke<E: Encode>(
        &self,
        name: impl Into<LambdaName>,
        payload: E,
    ) -> Result<InvokeResponse, InvokeError<E::Error>> {
        let (function_name, qualifier) = name.into().into_inner();
        let request = host::aws_lambda::InvokeRequest {
            function_name,
            qualifier,
            payload: Some(
                payload
                    .try_serialize()
                    .map_err(|e| InvokeError::EncodeFailed { cause: e })?
                    .into(),
            ),
            invocation_type: host::aws_lambda::InvocationType::RequestResponse(
                host::aws_lambda::InvokeSynchronousParameters {
                    log_type: None,
                    client_context: None,
                },
            ),
        };
        let output = self.client.invoke(&request)?;

        Ok(InvokeResponse {
            status_code: output.status_code,
            payload: output.payload,
        })
    }
}

/// Result from Lambda
pub struct InvokeResponse {
    /// The status code of the response
    status_code: i32,
    /// The payload of the response
    payload: Option<Vec<u8>>,
}

/// An error occurred when extracting the Lambda response.
#[derive(Debug, thiserror::Error)]
pub enum ResponseExtractError<E: ExtractError> {
    /// An error occurred when calling the provided extract method.
    Extract {
        /// The underlying error.
        cause: E,
    },
    /// The response was missing a payload.
    MissingPayload,
}

impl InvokeResponse {
    /// Get the status code of the response
    pub fn status_code(&self) -> i32 {
        self.status_code
    }

    /// Take the payload of the response
    ///
    /// This consumes the payload; if you call it again, it will return None.
    pub fn take_payload(&mut self) -> Option<Vec<u8>> {
        self.payload.take()
    }

    /// Take the payload of the response and decode it.
    ///
    /// This consumes the payload; if you call it again, it will return an Error.
    pub fn extract<E: Extract>(&mut self) -> Result<E, ResponseExtractError<E::Error>> {
        let payload = self
            .take_payload()
            .ok_or_else(|| ResponseExtractError::MissingPayload)?;
        E::extract(payload).map_err(|e| ResponseExtractError::Extract { cause: e })
    }
}

/// Identifier for a Lambda function
pub enum LambdaName {
    /// Lambda function name
    Name(String),
    /// Lambda function ARN
    Qualified {
        /// Name of the lambda function
        name: String,
        /// Version or alias of the lambda function
        qualifier: String,
    },
}
impl LambdaName {
    fn into_inner(self) -> (String, Option<String>) {
        match self {
            LambdaName::Name(name) => (name, None),
            LambdaName::Qualified { name, qualifier } => (name, Some(qualifier)),
        }
    }
}
impl From<String> for LambdaName {
    fn from(name: String) -> Self {
        LambdaName::Name(name)
    }
}
impl From<&str> for LambdaName {
    fn from(name: &str) -> Self {
        LambdaName::Name(name.to_string())
    }
}
impl From<(String, String)> for LambdaName {
    fn from((name, qualifier): (String, String)) -> Self {
        LambdaName::Qualified { name, qualifier }
    }
}
impl From<(&str, String)> for LambdaName {
    fn from((name, qualifier): (&str, String)) -> Self {
        LambdaName::Qualified {
            name: name.to_string(),
            qualifier,
        }
    }
}
impl From<(String, &str)> for LambdaName {
    fn from((name, qualifier): (String, &str)) -> Self {
        LambdaName::Qualified {
            name,
            qualifier: qualifier.to_string(),
        }
    }
}
impl From<(&str, &str)> for LambdaName {
    fn from((name, qualifier): (&str, &str)) -> Self {
        LambdaName::Qualified {
            name: name.to_string(),
            qualifier: qualifier.to_string(),
        }
    }
}
