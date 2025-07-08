//! Host interfaces for working with Momento Cache apis

use std::time::Duration;

use crate::encoding::{Encode, Extract};
use momento_functions_wit::host::momento::functions::cache_scalar;

/// An error occurred when setting a value in the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheSetError<E: Encode> {
    /// The provided value could not be encoded.
    #[error("Failed to encode value.")]
    EncodeFailed {
        /// The underlying encoding error.
        cause: E::Error,
    },
    /// An error occurred when calling the host cache function.
    #[error(transparent)]
    CacheError(#[from] cache_scalar::Error),
}

/// An error occurred when getting a value from the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheGetError<E: Extract> {
    /// The value could not be extracted with the provided implementation.
    #[error("Failed to extract value.")]
    ExtractFailed {
        /// The underlying error.
        cause: E::Error,
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
pub fn get<T: Extract>(key: impl AsRef<[u8]>) -> Result<Option<T>, CacheGetError<T>> {
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
) -> Result<(), CacheSetError<E>> {
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
