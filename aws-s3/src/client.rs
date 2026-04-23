use crate::wit::momento::aws_s3::aws_s3::{self as aws_s3};
use momento_functions_aws_auth::CredentialsProvider;
use momento_functions_bytes::{
    Data,
    encoding::{Encode, EncodeError, Extract, ExtractError},
};

/// S3 client for host interfaces.
///
/// This client uses Momento's host-provided AWS communication channel, which
/// is kept hot at all times. When your Function has not run in several days or more,
/// the channel is still hot and ready, keeping your Function invocations predictable
/// even when your demand is unpredictable.
pub struct S3Client {
    client: aws_s3::Client,
}

/// An error occurred while putting an object to S3.
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
    /// An error occurred when calling the host S3 interface.
    #[error(transparent)]
    S3Error(#[from] aws_s3::S3Error),
}

/// An error occurred while getting an object from S3.
#[derive(Debug, thiserror::Error)]
pub enum S3GetError<E>
where
    E: ExtractError,
{
    /// The value could not be extracted with the provided implementation.
    #[error("Failed to extract value.")]
    ExtractFailed {
        /// The underlying extract error.
        cause: E,
    },
    /// An error occurred when calling the host S3 interface.
    #[error(transparent)]
    S3Error(#[from] aws_s3::S3Error),
}

/// A request to put an object into an S3 bucket.
///
/// Construct with [`PutObjectRequest::new`] and add user-defined metadata
/// via [`PutObjectRequest::with_metadata`] or
/// [`PutObjectRequest::with_metadata_entry`].
pub struct PutObjectRequest<E: Encode> {
    bucket: String,
    key: String,
    body: E,
    metadata: Vec<(String, String)>,
}

impl<E: Encode> PutObjectRequest<E> {
    /// Create a new put request with no metadata.
    pub fn new(bucket: impl Into<String>, key: impl Into<String>, body: E) -> Self {
        Self {
            bucket: bucket.into(),
            key: key.into(),
            body,
            metadata: Vec::new(),
        }
    }

    /// Attach user-defined metadata entries to the request.
    ///
    /// Entries are sent as `x-amz-meta-*` headers and stored alongside the
    /// object. Existing entries are preserved; duplicate keys are not merged.
    pub fn with_metadata<K, V>(mut self, metadata: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.metadata
            .extend(metadata.into_iter().map(|(k, v)| (k.into(), v.into())));
        self
    }

    /// Attach a single user-defined metadata entry to the request.
    pub fn with_metadata_entry(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }
}

/// The response from a successful S3 get.
pub struct GetObjectResponse<T> {
    /// The decoded object body.
    pub body: T,
    /// User-defined metadata stored alongside the object.
    pub metadata: Vec<(String, String)>,
}

/// The response from a successful S3 put.
pub struct PutObjectResponse {
    /// Entity tag assigned by S3 to the stored object.
    pub etag: Option<String>,
    /// Version identifier assigned by S3 when bucket versioning is enabled.
    pub version_id: Option<String>,
    /// Expiration metadata returned by S3, when a lifecycle rule applies.
    pub expiration: Option<String>,
}

impl S3Client {
    /// Create a new S3 client.
    pub fn new(credentials: &CredentialsProvider) -> Self {
        Self {
            client: aws_s3::Client::new(credentials),
        }
    }

    /// Put an object into an S3 bucket.
    ///
    /// Build the request with [`PutObjectRequest::new`] and, if needed,
    /// [`PutObjectRequest::with_metadata`]. You can use strings, bytes, or
    /// structs that implement [`Encode`] as the body.
    pub fn put<E: Encode>(
        &self,
        request: PutObjectRequest<E>,
    ) -> Result<PutObjectResponse, S3PutError<E::Error>> {
        let body_data: Data = request
            .body
            .try_serialize()
            .map_err(|e| S3PutError::EncodeFailed { cause: e })?;
        let output = self
            .client
            .put(aws_s3::PutObjectRequest {
                bucket: request.bucket,
                key: request.key,
                body: body_data.into(),
                metadata: request.metadata,
            })
            .map_err(S3PutError::from)?;
        Ok(PutObjectResponse {
            etag: output.etag,
            version_id: output.version_id,
            expiration: output.expiration,
        })
    }

    /// Get an object from an S3 bucket along with its user-defined metadata.
    ///
    /// Returns `Ok(None)` if the object was not found.
    pub fn get<T: Extract>(
        &self,
        bucket: impl Into<String>,
        key: impl Into<String>,
    ) -> Result<Option<GetObjectResponse<T>>, S3GetError<T::Error>> {
        let output = self
            .client
            .get(&aws_s3::GetObjectRequest {
                bucket: bucket.into(),
                key: key.into(),
            })
            .map_err(S3GetError::from)?;
        if let Some(wit_data) = output.body {
            let data: Data = wit_data.into();
            T::extract(data)
                .map(|body| {
                    Some(GetObjectResponse {
                        body,
                        metadata: output.metadata,
                    })
                })
                .map_err(|e| S3GetError::ExtractFailed { cause: e })
        } else {
            Ok(None)
        }
    }
}
