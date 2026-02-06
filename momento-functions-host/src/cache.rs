//! Host interfaces for working with Momento Cache apis

use std::time::Duration;

use crate::encoding::{Encode, EncodeError, Extract, ExtractError};
use momento_functions_wit::host::momento::functions::cache_scalar;

pub use cache_scalar::GetWithHashFound;
pub use cache_scalar::GetWithHashResult;
pub use cache_scalar::SetIfCondition;
pub use cache_scalar::SetIfHashCondition;
pub use cache_scalar::SetIfHashResult;
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

/// An error occurred when getting a value with its hash from the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheGetWithHashError<E: ExtractError> {
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

/// An error occurred when conditionally setting a value based on hash comparison.
#[derive(thiserror::Error, Debug)]
pub enum CacheSetIfHashError<E: EncodeError> {
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

/// A value retrieved from the cache along with its hash.
pub struct GetWithHashValue<T> {
    /// The extracted value.
    pub value: T,
    /// The hash of the value.
    pub hash: Vec<u8>,
}

/// Get a value from the cache along with its hash.
///
/// The hash can be used with [`set_if_hash`] to perform conditional updates
/// based on whether the value has changed since it was read.
///
/// Examples:
/// ________
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::{CacheGetWithHashError, GetWithHashValue};
///
/// # fn f() -> Result<(), CacheGetWithHashError<Vec<u8>>> {
/// let result: Option<GetWithHashValue<Vec<u8>>> = cache::get_with_hash("my_key")?;
/// if let Some(entry) = result {
///     log::info!("Value: {:?}, Hash: {:?}", entry.value, entry.hash);
/// }
/// # Ok(()) }
/// ```
pub fn get_with_hash<T: Extract>(
    key: impl AsRef<[u8]>,
) -> Result<Option<GetWithHashValue<T>>, CacheGetWithHashError<T::Error>> {
    match cache_scalar::get_with_hash(key.as_ref())? {
        GetWithHashResult::Found(found) => {
            let value = T::extract(found.value)
                .map_err(|e| CacheGetWithHashError::ExtractFailed { cause: e })?;
            Ok(Some(GetWithHashValue {
                value,
                hash: found.hash,
            }))
        }
        GetWithHashResult::Missing => Ok(None),
    }
}

/// Conditionally set a value in the cache based on a hash comparison.
///
/// This is useful for optimistic concurrency control where you want to update
/// a value only if it hasn't changed since you last read it, without needing
/// to compare the full value.
///
/// Examples:
/// ________
/// Update only if the hash matches (value hasn't changed):
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::{CacheSetIfHashError, SetIfHashCondition, SetIfHashResult};
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheSetIfHashError<&'static str>> {
/// // First, get the current value and its hash
/// // let entry = cache::get_with_hash("my_key")?;
/// let previous_hash = vec![1, 2, 3]; // Hash from a previous get_with_hash call
///
/// let result = cache::set_if_hash(
///     "my_key",
///     b"new_value".to_vec(),
///     Duration::from_secs(60),
///     SetIfHashCondition::PresentAndHashEqual(previous_hash),
/// )?;
/// match result {
///     SetIfHashResult::Stored(new_hash) => {
///         log::info!("Value updated, new hash: {:?}", new_hash);
///     }
///     SetIfHashResult::NotStored => {
///         log::info!("Value was modified by another process");
///     }
/// }
/// # Ok(()) }
/// ```
pub fn set_if_hash<E: Encode>(
    key: impl AsRef<[u8]>,
    value: E,
    ttl: Duration,
    condition: SetIfHashCondition,
) -> Result<SetIfHashResult, CacheSetIfHashError<E::Error>> {
    cache_scalar::set_if_hash(
        key.as_ref(),
        &value
            .try_serialize()
            .map_err(|e| CacheSetIfHashError::EncodeFailed { cause: e })?
            .into(),
        saturate_ttl(ttl),
        &condition,
    )
    .map_err(Into::into)
}
