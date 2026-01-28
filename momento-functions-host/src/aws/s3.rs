//! Host interfaces for working with AWS S3
use momento_functions_wit::host::momento::host;
use momento_functions_wit::host::momento::host::aws_s3::S3Error;

use crate::encoding::{Encode, EncodeError, Extract, ExtractError};

use super::auth;

/// S3 client for host interfaces.
///
/// This client uses Momento's host-provided AWS communication channel, which
/// is kept hot at all times. When your Function has not run in several days or more,
/// the channel is still hot and ready, keeping your Function invocations predictable
/// even when your demand is unpredictable.
pub struct S3Client {
    client: host::aws_s3::Client,
}

/// An error occurred while putting an object to S3
#[derive(Debug, thiserror::Error)]
pub enum S3PutError<E>
where
    E: EncodeError,
{
    /// An error occurred while encoding the provided payload.
    #[error("Failed to encode payload.")]
    EncodeFailed {
        /// The underlying encode error.
        cause: E,
    },
    /// An error occurred when calling the host s3 interface.
    #[error(transparent)]
    S3Error(#[from] S3Error),
}

/// An error occurred while getting an object from S3
#[derive(Debug, thiserror::Error)]
pub enum S3GetError<E>
where
    E: ExtractError,
{
    /// The value could not be extracted with the provided implementation.
    #[error("Failed to extract value.")]
    ExtractFailed {
        /// The underlying encode error.
        cause: E,
    },
    /// An error occurred when calling the host s3 interface.
    #[error(transparent)]
    S3Error(#[from] S3Error),
}

impl S3Client {
    /// Create a new S3 client.
    ///
    /// ```rust,no-run
    /// # use momento_functions_host::aws::auth::AwsCredentialsProvider;
    /// # use momento_functions_host::aws::s3::S3Client;
    /// # use momento_functions_host::build_environment_aws_credentials;    /// #
    /// use momento_functions_wit::host::momento::host::aws_auth::AuthError;
    ///
    /// fn f() -> Result<(), AuthError> {
    /// let client = S3Client::new(
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
            client: host::aws_s3::Client::new(credentials.resource()),
        }
    }

    /// Put an object into an S3 bucket.
    ///
    /// You can use strings, bytes, or structs that are Serializable.
    ///
    /// Examples:
    /// ________
    /// ```rust,no-run
    /// use momento_functions_host::aws::s3::{S3PutError, S3Client};
    /// use momento_functions_host::encoding::Json;
    ///
    /// # fn f(client: &S3Client) -> Result<(), S3PutError<&str>> {
    /// // With a payload
    /// client.put(
    ///     "my-bucket",
    ///     "foo",
    ///     "bar",
    /// )?;
    ///
    /// // With literal bytes
    /// client.put(
    ///     "my-bucket",
    ///     "foo",
    ///     b"bar",
    /// )?;
    /// # Ok(())}
    /// ```
    /// ________
    /// With json-encoded payloads
    /// ```rust,no-run
    /// use momento_functions_host::aws::s3::{S3PutError, S3Client};
    /// use momento_functions_host::encoding::Json;
    ///
    /// #[derive(serde::Serialize)]
    /// struct MyStruct {
    ///     hello: String
    /// }
    ///
    /// # fn f(client: &S3Client) -> Result<(), S3EPutrror<Json<MyStruct>>> {
    ///
    /// // Just a request payload, encoded as JSON
    /// client.put(
    ///     "my-bucket",
    ///     "my-key",
    ///     Json(MyStruct { hello: "hello".to_string() }),
    /// )?;
    /// # Ok(())}
    /// ```
    pub fn put<E: Encode>(
        &self,
        bucket: impl Into<String>,
        key: impl Into<String>,
        body: E,
    ) -> Result<(), S3PutError<E::Error>> {
        let _output = self
            .client
            .put(&host::aws_s3::PutObjectRequest {
                bucket: bucket.into(),
                key: key.into(),
                body: body
                    .try_serialize()
                    .map_err(|e| S3PutError::EncodeFailed { cause: e })?
                    .into(),
            })
            .map_err(S3PutError::from)?;
        Ok(())
    }

    /// Get an object from an S3 bucket.
    ///
    /// The output's body is wrapped in an `Option`, with `None` indicating the object
    /// was not found with the given bucket and key.
    ///
    /// Examples:
    /// ________
    /// ```rust
    /// use momento_functions_host::aws::s3::{S3GetError, S3Client};
    /// use momento_functions_host::encoding::Json;
    ///
    /// # fn f(client: &S3Client) -> Result<(), S3GetError<&str>> {
    /// let my_value: Option<String> = match client.get(
    ///     "my-bucket",
    ///     "foo",
    /// )?;
    ///
    /// let another_value: Option<Vec<u8>> = client.get(
    ///     "my-bucket",
    ///     "bar",
    /// )?;
    /// # Ok(())}
    /// ```
    /// ________
    /// With json-encoded payloads
    /// ```rust
    /// use momento_functions_host::aws::s3::{S3GetError, S3Client};
    /// use momento_functions_host::encoding::Json;
    ///
    /// #[derive(serde::Serialize)]
    /// struct MyStruct {
    ///     hello: String
    /// }
    ///
    /// # fn f(client: &S3Client) -> Result<(), S3GetError<Json<MyStruct>>> {
    ///
    /// let maybe_struct: Option<MyStruct> = match client.get(
    ///     "my-bucket",
    ///     "my-key",
    /// )? {
    ///     Some(Json(my_struct)) => {
    ///       Some(my_struct)  
    ///     }
    ///     // Not found
    ///     None => None,
    /// };
    /// # Ok(())}
    /// ```
    pub fn get<T: Extract>(
        &self,
        bucket: impl Into<String>,
        key: impl Into<String>,
    ) -> Result<Option<T>, S3GetError<T::Error>> {
        let output = self
            .client
            .get(&host::aws_s3::GetObjectRequest {
                bucket: bucket.into(),
                key: key.into(),
            })
            .map_err(S3GetError::from)?;
        if let Some(body) = output.body {
            let value = T::extract(body).map_err(|e| S3GetError::ExtractFailed { cause: e })?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
}
