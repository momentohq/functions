//! Host interfaces for working with Momento Cache apis

use std::time::Duration;

use crate::encoding::{Encode, EncodeError, Extract, ExtractError};
use momento_functions_wit::host::momento::functions::cache_scalar;

pub use cache_scalar::SetIfCondition;
pub use cache_scalar::SetIfResult;

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
/// Get a value from the cache.
///
/// Examples:
/// ________
/// Bytes:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::CacheGetError;
///
/// # fn f() -> Result<(), CacheGetError<Vec<u8>>> {
/// let value: Option<Vec<u8>> = cache::get("my_key")?;
/// # Ok(()) }
/// ```
/// ________
/// Json:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::CacheGetError;
/// use momento_functions_host::encoding::Json;
///
/// #[derive(serde::Deserialize)]
/// struct MyStruct {
///   message: String
/// }
///
/// # fn f() -> Result<(), CacheGetError<Json<MyStruct>>> {
/// let value: Option<Json<MyStruct>> = cache::get("my_key")?;
/// # Ok(()) }
/// ```
pub fn get<T: Extract>(key: impl AsRef<[u8]>) -> Result<Option<T>, CacheGetError<T::Error>> {
    match cache_scalar::get(key.as_ref())? {
        Some(v) => T::extract(v)
            .map(Some)
            .map_err(|e| CacheGetError::ExtractFailed { cause: e }),
        None => Ok(None),
    }
}

/// Set a value in the cache with a time-to-live.
///
/// Examples:
/// ________
/// Bytes:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::CacheSetError;
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheSetError<&'static str>> {
/// cache::set(
///     "my_key",
///     b"hello".to_vec(),
///     Duration::from_secs(60),
/// )?;
/// # Ok(()) }
/// ```
/// ________
/// Json:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::CacheSetError;
/// # use std::time::Duration;
/// use momento_functions_host::encoding::Json;
///
/// #[derive(serde::Serialize)]
/// struct MyStruct {
///    hello: String
/// }
///
/// # fn f() -> Result<(), CacheSetError<Json<MyStruct>>> {
/// cache::set(
///     "my_key",
///     Json(MyStruct { hello: "hello".to_string() }),
///     Duration::from_secs(60),
/// )?;
/// # Ok(()) }
/// ```
pub fn set<E: Encode>(
    key: impl AsRef<[u8]>,
    value: E,
    ttl: Duration,
) -> Result<(), CacheSetError<E::Error>> {
    cache_scalar::set(
        key.as_ref(),
        &value
            .try_serialize()
            .map_err(|e| CacheSetError::EncodeFailed { cause: e })?
            .into(),
        saturate_ttl(ttl),
    )
    .map_err(Into::into)
}

fn saturate_ttl(ttl: Duration) -> u64 {
    ttl.as_millis().clamp(0, u64::MAX as u128) as u64
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

/// An error occurred when deleting a value from the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheDeleteError {
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_scalar::Error),
}

/// Conditionally set a value in the cache based on a condition.
///
/// Examples:
/// ________
/// Set only if absent:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::{CacheSetIfError, SetIfCondition, SetIfResult};
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheSetIfError<&'static str>> {
/// let result: SetIfResult = cache::set_if(
///     "my_key",
///     b"hello".to_vec(),
///     Duration::from_secs(60),
///     SetIfCondition::Absent,
/// )?;
/// match result {
///     SetIfResult::Stored => {
///         // Do something
///     },
///     SetIfResult::NotStored => {
///         // Do something else
///     },
/// }
/// # Ok(()) }
/// ```
/// ________
/// Set only if present:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::{CacheSetIfError, SetIfCondition, SetIfResult};
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheSetIfError<&'static str>> {
/// let result = cache::set_if(
///     "my_key",
///     b"updated".to_vec(),
///     Duration::from_secs(60),
///     SetIfCondition::Present,
/// )?;
/// # Ok(()) }
/// ```
/// ________
/// Set only if equal to a specific value:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::{CacheSetIfError, SetIfCondition, SetIfResult};
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheSetIfError<&'static str>> {
/// let result = cache::set_if(
///     "my_key",
///     b"new_value".to_vec(),
///     Duration::from_secs(60),
///     SetIfCondition::Equal(b"old_value".to_vec()),
/// )?;
/// # Ok(()) }
/// ```
pub fn set_if<E: Encode>(
    key: impl AsRef<[u8]>,
    value: E,
    ttl: Duration,
    condition: SetIfCondition,
) -> Result<SetIfResult, CacheSetIfError<E::Error>> {
    cache_scalar::set_if(
        key.as_ref(),
        &value
            .try_serialize()
            .map_err(|e| CacheSetIfError::EncodeFailed { cause: e })?
            .into(),
        saturate_ttl(ttl),
        &condition,
    )
    .map_err(Into::into)
}

/// Delete a value from the cache.
///
/// Note: This operation is idempotent. Deleting a key that does not exist
/// will not return an error.
///
/// Examples:
/// ________
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::CacheDeleteError;
///
/// # fn f() -> Result<(), CacheDeleteError> {
/// cache::delete("my_key")?;
/// # Ok(()) }
/// ```
pub fn delete(key: impl AsRef<[u8]>) -> Result<(), CacheDeleteError> {
    cache_scalar::delete(key.as_ref()).map_err(Into::into)
}
