use std::time::Duration;

use momento_functions_bytes::{
    Data,
    encoding::{Encode, Extract},
};

use crate::{
    CacheSetError, SetIfCondition,
    errors::{
        CacheDeleteError, CacheGetError, CacheGetWithHashError, CacheSetIfError,
        CacheSetIfHashError,
    },
    set_if::{ConditionalSetResult, GetWithHashValue, SetIfHashCondition, SetIfHashResult},
    wit::momento::cache_scalar::cache_scalar,
};

/// Get a value from the cache.
///
/// Examples:
/// ________
/// Bytes:
/// ```rust,no_run
/// use momento_functions_cache::get;
///
/// match get::<Vec<u8>>("my_key") {
///     Ok(Some(value)) => { /* use value */ }
///     Ok(None) => { /* key not found */ }
///     Err(e) => eprintln!("cache get failed: {e}"),
/// }
/// ```
/// ________
/// Json:
/// ```rust,no_run
/// use momento_functions_cache::get;
/// use momento_functions_bytes::encoding::Json;
///
/// #[derive(serde::Deserialize)]
/// struct MyStruct {
///   message: String
/// }
///
/// match get::<Json<MyStruct>>("my_key") {
///     Ok(Some(Json(value))) => { /* use value */ }
///     Ok(None) => { /* key not found */ }
///     Err(e) => eprintln!("cache get failed: {e}"),
/// }
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
/// ```rust,no_run
/// use momento_functions_cache::set;
/// # use std::time::Duration;
/// match set(
///     "my_key",
///     b"hello".to_vec(),
///     Duration::from_secs(60),
/// ) {
///     Ok(()) => {}
///     Err(e) => eprintln!("cache set failed: {e}"),
/// }
/// ```
/// ________
/// Json:
/// ```rust,no_run
/// use momento_functions_cache::set;
/// # use std::time::Duration;
/// use momento_functions_bytes::encoding::Json;
///
/// #[derive(serde::Serialize)]
/// struct MyStruct {
///    hello: String
/// }
///
/// match set(
///     "my_key",
///     Json(MyStruct { hello: "hello".to_string() }),
///     Duration::from_secs(60),
/// ) {
///     Ok(()) => {}
///     Err(e) => eprintln!("cache set failed: {e}"),
/// }
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
/// ```rust,no_run
/// # use momento_functions_cache::set_if;
/// # use momento_functions_cache::{ConditionalSetResult, SetIfCondition};
/// # use std::time::Duration;
/// match set_if(
///     "my_key",
///     b"hello".to_vec(),
///     Duration::from_secs(60),
///     SetIfCondition::Absent,
/// ) {
///     Ok(ConditionalSetResult::Stored(_)) => {
///         // Do something
///     }
///     Ok(ConditionalSetResult::NotStored) => {
///         // Do something else
///     }
///     Err(e) => eprintln!("set_if failed: {e}"),
/// }
/// ```
/// ________
/// Set only if present:
/// ```rust,no_run
/// # use momento_functions_cache::set_if;
/// # use momento_functions_cache::SetIfCondition;
/// # use std::time::Duration;
/// match set_if(
///     "my_key",
///     b"updated".to_vec(),
///     Duration::from_secs(60),
///     SetIfCondition::Present,
/// ) {
///     Ok(result) => { /* inspect result */ }
///     Err(e) => eprintln!("set_if failed: {e}"),
/// }
/// ```
/// ________
/// Set only if equal to a specific value:
/// ```rust,no_run
/// # use momento_functions_cache::set_if;
/// # use momento_functions_cache::SetIfCondition;
/// # use std::time::Duration;
/// match set_if(
///     "my_key",
///     b"new_value".to_vec(),
///     Duration::from_secs(60),
///     SetIfCondition::Equal("old_value".into()),
/// ) {
///     Ok(result) => { /* inspect result */ }
///     Err(e) => eprintln!("set_if failed: {e}"),
/// }
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

/// Delete a value from the cache.
///
/// Note: This operation is idempotent. Deleting a key that does not exist
/// will not return an error.
///
/// Examples:
/// ________
/// ```rust,no_run
/// use momento_functions_cache::delete;
///
/// match delete("my_key") {
///     Ok(()) => {}
///     Err(e) => eprintln!("cache delete failed: {e}"),
/// }
/// ```
pub fn delete(key: impl Into<Data>) -> Result<(), CacheDeleteError> {
    cache_scalar::delete(key.into().into()).map_err(Into::into)
}

/// Get a value from the cache along with its hash.
///
/// The hash can be used with [`set_if_hash`] to perform conditional updates
/// based on whether the value has changed since it was read.
///
/// Examples:
/// ________
/// ```rust,no_run
/// use momento_functions_cache::{get_with_hash, GetWithHashValue};
///
/// match get_with_hash::<Vec<u8>>("my_key") {
///     Ok(Some(entry)) => {
///         // use entry.value and entry.hash
///     }
///     Ok(None) => { /* key not found */ }
///     Err(e) => eprintln!("get_with_hash failed: {e}"),
/// }
/// ```
pub fn get_with_hash<T: Extract>(
    key: impl Into<Data>,
) -> Result<Option<GetWithHashValue<T>>, CacheGetWithHashError<T::Error>> {
    match cache_scalar::get_with_hash(key.into().into())? {
        cache_scalar::GetWithHashResult::Found(found) => {
            let value = T::extract(Data::from(found.value))
                .map_err(|e| CacheGetWithHashError::ExtractFailed { cause: e })?;
            Ok(Some(GetWithHashValue {
                value,
                hash: Data::from(found.hash).into_bytes(),
            }))
        }
        cache_scalar::GetWithHashResult::Missing => Ok(None),
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
/// ```rust,no_run
/// use momento_functions_cache::{SetIfHashCondition, SetIfHashResult};
/// use momento_functions_cache::set_if_hash;
/// # use std::time::Duration;
/// // First, get the current value and its hash
/// // let entry = get_with_hash("my_key")?;
/// let previous_hash = vec![1, 2, 3]; // Hash from a previous get_with_hash call
///
/// match set_if_hash(
///     "my_key",
///     b"new_value".to_vec(),
///     Duration::from_secs(60),
///     SetIfHashCondition::PresentAndHashEqual(previous_hash.into()),
/// ) {
///     Ok(SetIfHashResult::Stored(new_hash)) => {
///         // value updated, use new_hash
///     }
///     Ok(SetIfHashResult::NotStored) => {
///         // value was modified by another process
///     }
///     Err(e) => eprintln!("set_if_hash failed: {e}"),
/// }
/// ```
pub fn set_if_hash<E: Encode>(
    key: impl Into<Data>,
    value: E,
    ttl: Duration,
    condition: SetIfHashCondition,
) -> Result<SetIfHashResult, CacheSetIfHashError<E::Error>> {
    cache_scalar::set_if_hash(
        key.into().into(),
        value
            .try_serialize()
            .map_err(|e| CacheSetIfHashError::EncodeFailed { cause: e })?
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
