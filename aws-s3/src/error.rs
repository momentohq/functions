use crate::wit::momento::aws_s3::aws_s3;

/// An error returned by the host S3 interface.
#[derive(Debug, thiserror::Error)]
pub enum S3Error {
    /// The request was not authorized. Not retryable without changing credentials or permissions.
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    /// The request was malformed. Not retryable without changing the request.
    #[error("malformed request: {0}")]
    Malformed(String),
    /// The request failed for some other reason.
    #[error("S3 request failed: {0}")]
    Other(String),
    /// S3 is throttling requests (503 SlowDown). Transient; back off and retry.
    #[error("S3 throttled the request: {0}")]
    Throttled(String),
}

impl From<aws_s3::S3Error> for S3Error {
    fn from(e: aws_s3::S3Error) -> Self {
        match e {
            aws_s3::S3Error::Unauthorized(s) => S3Error::Unauthorized(s),
            aws_s3::S3Error::Malformed(s) => S3Error::Malformed(s),
            aws_s3::S3Error::Other(s) => S3Error::Other(s),
            aws_s3::S3Error::Throttled(s) => S3Error::Throttled(s),
        }
    }
}
