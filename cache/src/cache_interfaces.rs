use std::time::Duration;

use momento_functions_bytes::{
    Data,
    encoding::{Encode, Extract},
};

use crate::{
    CacheSetError, SetIfCondition,
    errors::{CacheGetError, CacheSetIfError},
    set_if::ConditionalSetResult,
    wit::momento::cache_scalar::cache_scalar,
};

/// Get a value from the cache.
///
/// Examples:
/// ________
/// Bytes:
/// ```rust
/// use momento_functions_cache::CacheGetError;
/// use momento_functions_cache::get;
/// # use std::convert::Infallible;
///
/// # fn f() -> Result<(), CacheGetError<Infallible>> {
/// let value: Option<Vec<u8>> = get("my_key")?;
/// # Ok(()) }
/// ```
/// ________
/// Json:
/// ```rust
/// use momento_functions_cache::CacheGetError;
/// use momento_functions_cache::get;
/// use momento_functions_bytes::encoding::Json;
///
/// #[derive(serde::Deserialize)]
/// struct MyStruct {
///   message: String
/// }
///
/// # fn f() -> Result<(), CacheGetError<serde_json::Error>> {
/// let value: Option<Json<MyStruct>> = get("my_key")?;
/// # Ok(()) }
/// ```
pub fn get<T: Extract>(key: impl Into<Data>) -> Result<Option<T>, CacheGetError<T::Error>> {
    match cache_scalar::get(key.into().into())? {
        Some(v) => T::extract(v.into())
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
/// use momento_functions_cache::set;
/// use momento_functions_cache::CacheSetError;
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheSetError<std::convert::Infallible>> {
/// set(
///     "my_key",
///     b"hello".to_vec(),
///     Duration::from_secs(60),
/// )?;
/// # Ok(()) }
/// ```
/// ________
/// Json:
/// ```rust
/// use momento_functions_cache::set;
/// use momento_functions_cache::CacheSetError;
/// # use std::time::Duration;
/// use momento_functions_bytes::encoding::Json;
///
/// #[derive(serde::Serialize)]
/// struct MyStruct {
///    hello: String
/// }
///
/// # fn f() -> Result<(), CacheSetError<serde_json::Error>> {
/// set(
///     "my_key",
///     Json(MyStruct { hello: "hello".to_string() }),
///     Duration::from_secs(60),
/// )?;
/// # Ok(()) }
/// ```
pub fn set<E: Encode>(
    key: impl Into<Data>,
    value: E,
    ttl: Duration,
) -> Result<(), CacheSetError<E::Error>> {
    cache_scalar::set(
        key.into().into(),
        value
            .try_serialize()
            .map_err(|e| CacheSetError::EncodeFailed { cause: e })?
            .into(),
        saturate_ttl(ttl),
    )
    .map_err(Into::into)
}

/// Conditionally set a value in the cache based on a condition.
///
/// Examples:
/// ________
/// Set only if absent:
/// ```rust
/// # use momento_functions_cache::set_if;
/// # use momento_functions_cache::{ConditionalSetResult, CacheSetIfError, SetIfCondition};
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheSetIfError<std::convert::Infallible>> {
/// let result: ConditionalSetResult<()> = set_if(
///     "my_key",
///     b"hello".to_vec(),
///     Duration::from_secs(60),
///     SetIfCondition::Absent,
/// )?;
/// match result {
///     ConditionalSetResult::Stored(_) => {
///         // Do something
///     },
///     ConditionalSetResult::NotStored => {
///         // Do something else
///     },
/// }
/// # Ok(()) }
/// ```
/// ________
/// Set only if present:
/// ```rust
/// # use momento_functions_cache::set_if;
/// # use momento_functions_cache::{CacheSetIfError, SetIfCondition, ConditionalSetResult};
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheSetIfError<std::convert::Infallible>> {
/// let result = set_if(
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
/// # use momento_functions_cache::set_if;
/// # use momento_functions_cache::{CacheSetIfError, SetIfCondition, ConditionalSetResult};
/// # use std::time::Duration;
///
/// # fn f() -> Result<(), CacheSetIfError<std::convert::Infallible>> {
/// let result = set_if(
///     "my_key",
///     b"new_value".to_vec(),
///     Duration::from_secs(60),
///     SetIfCondition::Equal("old_value".into()),
/// )?;
/// # Ok(()) }
/// ```
pub fn set_if<E: Encode>(
    key: impl Into<Data>,
    value: E,
    ttl: Duration,
    condition: SetIfCondition,
) -> Result<ConditionalSetResult<()>, CacheSetIfError<E::Error>> {
    cache_scalar::set_if(
        key.into().into(),
        value
            .try_serialize()
            .map_err(|e| CacheSetIfError::EncodeFailed { cause: e })?
            .into(),
        saturate_ttl(ttl),
        condition.into(),
    )
    .map(Into::into)
    .map_err(Into::into)
}

fn saturate_ttl(ttl: Duration) -> u64 {
    ttl.as_millis().min(u64::MAX as u128) as u64
}
