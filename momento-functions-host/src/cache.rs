//! Host interfaces for working with Momento Cache apis

use std::time::Duration;

use crate::encoding::{Encode, EncodeError, Extract, ExtractError};
use momento_functions_wit::host::momento::functions::cache_list;
use momento_functions_wit::host::momento::functions::cache_scalar;

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

/// An error occurred when pushing a value to a list in the cache.
#[derive(thiserror::Error, Debug)]
pub enum CacheListPushError<E: EncodeError> {
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

/// Push a value to the front of a list in the cache.
///
/// Returns the length of the list after the push operation.
///
/// Examples:
/// ________
/// Bytes:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::CacheListPushError;
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheListPushError<&'static str>> {
/// let list_length = cache::list_push_front(
///     "my_list",
///     b"hello".to_vec(),
///     Duration::from_secs(60),
///     true,
///     100,
/// )?;
/// # Ok(()) }
/// ```
/// ________
/// Json:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::CacheListPushError;
/// # use std::time::Duration;
/// use momento_functions_host::encoding::Json;
///
/// #[derive(serde::Serialize)]
/// struct MyStruct {
///    hello: String
/// }
///
/// # fn f() -> Result<(), CacheListPushError<Json<MyStruct>>> {
/// let list_length = cache::list_push_front(
///     "my_list",
///     Json(MyStruct { hello: "hello".to_string() }),
///     Duration::from_secs(60),
///     true,
///     100,
/// )?;
/// # Ok(()) }
/// ```
pub fn list_push_front<E: Encode>(
    list_name: impl AsRef<[u8]>,
    value: E,
    ttl: Duration,
    refresh_ttl: bool,
    truncate_back_to_size: u32,
) -> Result<u32, CacheListPushError<E::Error>> {
    cache_list::list_push_front(
        list_name.as_ref(),
        &value
            .try_serialize()
            .map_err(|e| CacheListPushError::EncodeFailed { cause: e })?
            .into(),
        saturate_ttl(ttl),
        refresh_ttl,
        truncate_back_to_size,
    )
    .map_err(Into::into)
}

/// Push a value to the back of a list in the cache.
///
/// Returns the length of the list after the push operation.
///
/// Examples:
/// ________
/// Bytes:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::CacheListPushError;
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheListPushError<&'static str>> {
/// let list_length = cache::list_push_back(
///     "my_list",
///     b"hello".to_vec(),
///     Duration::from_secs(60),
///     true,
///     100,
/// )?;
/// # Ok(()) }
/// ```
/// ________
/// Json:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::CacheListPushError;
/// # use std::time::Duration;
/// use momento_functions_host::encoding::Json;
///
/// #[derive(serde::Serialize)]
/// struct MyStruct {
///    hello: String
/// }
///
/// # fn f() -> Result<(), CacheListPushError<Json<MyStruct>>> {
/// let list_length = cache::list_push_back(
///     "my_list",
///     Json(MyStruct { hello: "hello".to_string() }),
///     Duration::from_secs(60),
///     true,
///     100,
/// )?;
/// # Ok(()) }
/// ```
pub fn list_push_back<E: Encode>(
    list_name: impl AsRef<[u8]>,
    value: E,
    ttl: Duration,
    refresh_ttl: bool,
    truncate_back_to_size: u32,
) -> Result<u32, CacheListPushError<E::Error>> {
    cache_list::list_push_back(
        list_name.as_ref(),
        &value
            .try_serialize()
            .map_err(|e| CacheListPushError::EncodeFailed { cause: e })?
            .into(),
        saturate_ttl(ttl),
        refresh_ttl,
        truncate_back_to_size,
    )
    .map_err(Into::into)
}

/// Pop a value from the front of a list in the cache.
///
/// Returns `None` if the list does not exist, or `Some((value, list_length))` if a value was popped.
/// The `list_length` is the length of the list after the pop operation.
///
/// Examples:
/// ________
/// Bytes:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::CacheListPopError;
///
/// # fn f() -> Result<(), CacheListPopError<Vec<u8>>> {
/// let result: Option<(Vec<u8>, u32)> = cache::list_pop_front("my_list")?;
/// # Ok(()) }
/// ```
/// ________
/// Json:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::CacheListPopError;
/// use momento_functions_host::encoding::Json;
///
/// #[derive(serde::Deserialize)]
/// struct MyStruct {
///   message: String
/// }
///
/// # fn f() -> Result<(), CacheListPopError<Json<MyStruct>>> {
/// let result: Option<(Json<MyStruct>, u32)> = cache::list_pop_front("my_list")?;
/// # Ok(()) }
/// ```
pub fn list_pop_front<T: Extract>(
    list_name: impl AsRef<[u8]>,
) -> Result<Option<(T, u32)>, CacheListPopError<T::Error>> {
    match cache_list::list_pop_front(list_name.as_ref())? {
        cache_list::PopResponse::Found(pop_found) => {
            let value = T::extract(pop_found.value)
                .map_err(|e| CacheListPopError::ExtractFailed { cause: e })?;
            Ok(Some((value, pop_found.list_length)))
        }
        cache_list::PopResponse::Missing => Ok(None),
    }
}

/// Pop a value from the back of a list in the cache.
///
/// Returns `None` if the list does not exist, or `Some((value, list_length))` if a value was popped.
/// The `list_length` is the length of the list after the pop operation.
///
/// Examples:
/// ________
/// Bytes:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::CacheListPopError;
///
/// # fn f() -> Result<(), CacheListPopError<Vec<u8>>> {
/// let result: Option<(Vec<u8>, u32)> = cache::list_pop_back("my_list")?;
/// # Ok(()) }
/// ```
/// ________
/// Json:
/// ```rust
/// # use momento_functions_host::cache;
/// # use momento_functions_host::cache::CacheListPopError;
/// use momento_functions_host::encoding::Json;
///
/// #[derive(serde::Deserialize)]
/// struct MyStruct {
///   message: String
/// }
///
/// # fn f() -> Result<(), CacheListPopError<Json<MyStruct>>> {
/// let result: Option<(Json<MyStruct>, u32)> = cache::list_pop_back("my_list")?;
/// # Ok(()) }
/// ```
pub fn list_pop_back<T: Extract>(
    list_name: impl AsRef<[u8]>,
) -> Result<Option<(T, u32)>, CacheListPopError<T::Error>> {
    match cache_list::list_pop_back(list_name.as_ref())? {
        cache_list::PopResponse::Found(pop_found) => {
            let value = T::extract(pop_found.value)
                .map_err(|e| CacheListPopError::ExtractFailed { cause: e })?;
            Ok(Some((value, pop_found.list_length)))
        }
        cache_list::PopResponse::Missing => Ok(None),
    }
}
