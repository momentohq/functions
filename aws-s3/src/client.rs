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

impl S3Client {
    /// Create a new S3 client.
    pub fn new(credentials: &CredentialsProvider) -> Self {
        Self {
            client: aws_s3::Client::new(credentials),
        }
    }

    /// Put an object into an S3 bucket.
    ///
    /// You can use strings, bytes, or structs that implement [`Encode`].
    pub fn put<E: Encode>(
        &self,
        bucket: impl Into<String>,
        key: impl Into<String>,
        body: E,
    ) -> Result<(), S3PutError<E::Error>> {
        let body_data: Data = body
            .try_serialize()
            .map_err(|e| S3PutError::EncodeFailed { cause: e })?;
        self.client
            .put(aws_s3::PutObjectRequest {
                bucket: bucket.into(),
                key: key.into(),
                body: body_data.into(),
            })
            .map_err(S3PutError::from)?;
        Ok(())
    }

    /// Get an object from an S3 bucket.
    ///
    /// Returns `Ok(None)` if the object was not found.
    pub fn get<T: Extract>(
        &self,
        bucket: impl Into<String>,
        key: impl Into<String>,
    ) -> Result<Option<T>, S3GetError<T::Error>> {
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
                .map(Some)
                .map_err(|e| S3GetError::ExtractFailed { cause: e })
        } else {
            Ok(None)
        }
    }
}
