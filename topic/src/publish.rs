use momento_functions_bytes::Data;
use thiserror::Error;

use crate::wit::momento::topic::topic;

/// An error returned by a topic publish operation.
#[derive(Debug, Error)]
pub enum TopicError {
    /// An internal error occurred.
    #[error("internal error")]
    InternalError,
    /// The request was cancelled.
    #[error("request cancelled")]
    RequestCancelled,
    /// An invalid argument was provided.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    /// The request timed out.
    #[error("timeout")]
    Timeout,
    /// Permission was denied.
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    /// A limit was exceeded.
    #[error("limit exceeded: {0}")]
    LimitExceeded(String),
    /// A precondition was not met.
    #[error("failed precondition: {0}")]
    FailedPrecondition(String),
    /// The topic was not found.
    #[error("not found: {0}")]
    NotFound(String),
}

impl From<topic::Error> for TopicError {
    fn from(e: topic::Error) -> Self {
        match e {
            topic::Error::InternalError => TopicError::InternalError,
            topic::Error::RequestCancelled => TopicError::RequestCancelled,
            topic::Error::InvalidArgument(s) => TopicError::InvalidArgument(s),
            topic::Error::Timeout => TopicError::Timeout,
            topic::Error::PermissionDenied(s) => TopicError::PermissionDenied(s),
            topic::Error::LimitExceeded(s) => TopicError::LimitExceeded(s),
            topic::Error::FailedPrecondition(s) => TopicError::FailedPrecondition(s),
            topic::Error::NotFound(s) => TopicError::NotFound(s),
        }
    }
}

/// Publish a string message to a topic.
///
/// # Arguments
/// * `topic_name` - The name of the topic to publish to.
/// * `value` - The string message to publish.
///
/// # Examples
/// ________
/// Publish a message:
/// ```rust,no_run
/// use momento_functions_topic::{publish, TopicError};
///
/// # fn f() -> Result<(), TopicError> {
/// publish("my-topic", "hello world")?;
/// # Ok(()) }
/// ```
pub fn publish(topic_name: impl Into<String>, value: impl Into<String>) -> Result<(), TopicError> {
    topic::publish(&topic_name.into(), &value.into()).map_err(Into::into)
}

/// Publish a bytes message to a topic.
///
/// # Arguments
/// * `topic_name` - The name of the topic to publish to.
/// * `value` - The bytes message to publish.
///
/// # Examples
/// ________
/// Publish binary data:
/// ```rust,no_run
/// use momento_functions_topic::{publish_bytes, TopicError};
///
/// # fn f() -> Result<(), TopicError> {
/// publish_bytes("my-topic", b"binary data".to_vec())?;
/// # Ok(()) }
/// ```
pub fn publish_bytes(
    topic_name: impl Into<String>,
    value: impl Into<Data>,
) -> Result<(), TopicError> {
    topic::publish_bytes(&topic_name.into(), value.into().into()).map_err(Into::into)
}
