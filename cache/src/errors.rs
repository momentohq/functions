use momento_functions_bytes::encoding::{EncodeError, ExtractError};

use crate::wit::momento::cache_scalar::cache_scalar;

/// An error occurred when setting a value in the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheSetError<E: EncodeError> {
    /// The provided value could not be encoded.
    #[error("Failed to encode value.")]
    EncodeFailed {
        /// The underlying encoding error.
        cause: E,
    },
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_scalar::Error),
}

/// An error occurred when getting a value from the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheGetError<E: ExtractError> {
    /// The value could not be extracted with the provided implementation.
    #[error("Failed to extract value.")]
    ExtractFailed {
        /// The underlying error.
        cause: E,
    },
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_scalar::Error),
}

/// An error occurred when conditionally setting a value in the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheSetIfError<E: EncodeError> {
    /// The provided value could not be encoded.
    #[error("Failed to encode value.")]
    EncodeFailed {
        /// The underlying encoding error.
        cause: E,
    },
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_scalar::Error),
}
