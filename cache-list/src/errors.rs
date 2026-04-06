use momento_functions_bytes::encoding::{EncodeError, ExtractError};

use crate::wit::momento::cache_list::cache_list;

/// An error occurred when pushing a value to the back of a list in the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheListPushBackError<E: EncodeError> {
    /// The provided value could not be encoded.
    #[error("Failed to encode value.")]
    EncodeFailed {
        /// The underlying encoding error.
        cause: E,
    },
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_list::Error),
}

/// An error occurred when pushing a value to the front of a list in the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheListPushFrontError<E: EncodeError> {
    /// The provided value could not be encoded.
    #[error("Failed to encode value.")]
    EncodeFailed {
        /// The underlying encoding error.
        cause: E,
    },
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_list::Error),
}

/// An error occurred when popping a value from a list in the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheListPopError<E: ExtractError> {
    /// The value could not be extracted with the provided implementation.
    #[error("Failed to extract value.")]
    ExtractFailed {
        /// The underlying error.
        cause: E,
    },
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_list::Error),
}

/// An error occurred when erasing elements from a list in the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheListEraseError {
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_list::Error),
}

/// An error occurred when removing elements from a list in the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheListRemoveError {
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_list::Error),
}

/// An error occurred when fetching elements from a list in the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheListFetchError<E: ExtractError> {
    /// The value could not be extracted with the provided implementation.
    #[error("Failed to extract value.")]
    ExtractFailed {
        /// The underlying error.
        cause: E,
    },
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_list::Error),
}

/// An error occurred when getting the length of a list in the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheListLengthError {
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_list::Error),
}

/// An error occurred when concatenating values to a list in the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheListConcatenateError<E: EncodeError> {
    /// The provided value could not be encoded.
    #[error("Failed to encode value.")]
    EncodeFailed {
        /// The underlying encoding error.
        cause: E,
    },
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_list::Error),
}

/// An error occurred when retaining elements in a list in the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheListRetainError {
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_list::Error),
}
