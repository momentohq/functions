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

/// Options for S3 object operations.
///
/// Used with [`S3Client::put_with_options`] and [`S3Client::get_with_options`] to specify
/// optional content metadata for S3 objects.
///
/// All fields default to `None`, so you only need to set the ones you care about.
///
/// # Examples
///
/// ```rust
/// use momento_functions_host::aws::s3::S3ObjectOptions;
///
/// // Set only content-type
/// let options = S3ObjectOptions {
///     content_type: Some("application/json".to_string()),
///     ..Default::default()
/// };
///
/// // Set both content-type and content-encoding
/// let options = S3ObjectOptions {
///     content_type: Some("application/json".to_string()),
///     content_encoding: Some("gzip".to_string()),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Default, Clone)]
pub struct S3ObjectOptions {
    /// The MIME type of the object (e.g. `"application/json"`, `"image/png"`).
    pub content_type: Option<String>,
    /// The encoding of the object (e.g. `"gzip"`, `"br"`).
    pub content_encoding: Option<String>,
}

/// The result of a [`S3Client::get_with_options`] call.
///
/// Contains the extracted value along with any content metadata returned by S3.
#[derive(Debug)]
pub struct S3GetOutput<T> {
    /// The extracted value.
    pub value: T,
    /// The content type of the object, if set.
    pub content_type: Option<String>,
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
    /// If you need to set content-type or content-encoding, use
    /// [`put_with_options`](S3Client::put_with_options) instead.
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
    /// # fn f(client: &S3Client) -> Result<(), S3PutError<Json<MyStruct>>> {
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
        self.put_with_options(bucket, key, body, S3ObjectOptions::default())
    }

    /// Put an object into an S3 bucket with additional options.
    ///
    /// This is the same as [`put`](S3Client::put), but allows you to specify
    /// content-type and content-encoding for the object.
    ///
    /// # Examples
    ///
    /// ```rust,no-run
    /// use momento_functions_host::aws::s3::{S3PutError, S3Client, S3ObjectOptions};
    ///
    /// # fn f(client: &S3Client) -> Result<(), S3PutError<&str>> {
    /// client.put_with_options(
    ///     "my-bucket",
    ///     "my-key",
    ///     b"compressed data",
    ///     S3ObjectOptions {
    ///         content_type: Some("application/octet-stream".to_string()),
    ///         content_encoding: Some("gzip".to_string()),
    ///         ..Default::default()
    ///     },
    /// )?;
    /// # Ok(())}
    /// ```
    pub fn put_with_options<E: Encode>(
        &self,
        bucket: impl Into<String>,
        key: impl Into<String>,
        body: E,
        options: S3ObjectOptions,
    ) -> Result<(), S3PutError<E::Error>> {
        let _output = self
            .client
            .put_extended(
                &host::aws_s3::PutObjectRequest {
                    bucket: bucket.into(),
                    key: key.into(),
                    body: body
                        .try_serialize()
                        .map_err(|e| S3PutError::EncodeFailed { cause: e })?
                        .into(),
                },
                &host::aws_s3::ObjectOptions {
                    content_type: options.content_type,
                    content_encoding: options.content_encoding,
                },
            )
            .map_err(S3PutError::from)?;
        Ok(())
    }

    /// Get an object from an S3 bucket.
    ///
    /// The output's body is wrapped in an `Option`, with `None` indicating the object
    /// was not found with the given bucket and key.
    ///
    /// If you need to set request options or retrieve content metadata like content-type,
    /// use [`get_with_options`](S3Client::get_with_options) instead.
    ///
    /// Examples:
    /// ________
    /// ```rust
    /// use momento_functions_host::aws::s3::{S3GetError, S3Client};
    /// use momento_functions_host::encoding::Json;
    ///
    /// # fn f(client: &S3Client) -> Result<(), S3GetError<&str>> {
    /// let my_value: Option<String> = client.get(
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
        Ok(self
            .get_with_options(bucket, key, S3ObjectOptions::default())?
            .map(|output| output.value))
    }

    /// Get an object from an S3 bucket with additional options.
    ///
    /// This is the same as [`get`](S3Client::get), but allows you to specify
    /// request options and returns content metadata (like content-type) alongside the value.
    ///
    /// Returns `Ok(None)` if the object was not found.
    ///
    /// # Examples
    ///
    /// ```rust,no-run
    /// use momento_functions_host::aws::s3::{S3GetError, S3Client, S3ObjectOptions};
    ///
    /// # fn f(client: &S3Client) -> Result<(), S3GetError<&str>> {
    /// let result = client.get_with_options::<Vec<u8>>(
    ///     "my-bucket",
    ///     "my-key",
    ///     S3ObjectOptions::default(),
    /// )?;
    ///
    /// if let Some(output) = result {
    ///     println!("content-type: {:?}", output.content_type);
    ///     println!("body length: {}", output.value.len());
    /// }
    /// # Ok(())}
    /// ```
    pub fn get_with_options<T: Extract>(
        &self,
        bucket: impl Into<String>,
        key: impl Into<String>,
        options: S3ObjectOptions,
    ) -> Result<Option<S3GetOutput<T>>, S3GetError<T::Error>> {
        let output = self
            .client
            .get_extended(
                &host::aws_s3::GetObjectRequest {
                    bucket: bucket.into(),
                    key: key.into(),
                },
                &host::aws_s3::ObjectOptions {
                    content_type: options.content_type,
                    content_encoding: options.content_encoding,
                },
            )
            .map_err(S3GetError::from)?;
        if let Some(body) = output.body {
            let value = T::extract(body).map_err(|e| S3GetError::ExtractFailed { cause: e })?;
            Ok(Some(S3GetOutput {
                value,
                content_type: output.content_type,
            }))
        } else {
            Ok(None)
        }
    }
}
