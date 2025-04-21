//! Host interfaces for working with Momento Cache apis

use std::time::Duration;

use momento_functions_wit::host::momento::functions::cache_scalar;

use crate::FunctionResult;

/// Get a value from the cache.
///
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::cache;
///
/// # fn f() -> FunctionResult<()> {
/// let value: Option<Vec<u8>> = cache::get("my_key")?;
/// # Ok(()) }
/// ```
pub fn get(key: impl AsRef<[u8]>) -> FunctionResult<Option<Vec<u8>>> {
    cache_scalar::get(key.as_ref()).map_err(Into::into)
}

/// Get a value from the cache and interpret the value as JSON.
///
/// On deserialization failure, an error is returned.
///
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::cache;
///
/// #[derive(serde::Deserialize)]
/// struct MyStruct {
///    hello: String
/// }
///
/// # fn f() -> FunctionResult<()> {
/// let value: Option<MyStruct> = cache::get_json("my_key")?;
/// # Ok(()) }
/// ```
pub fn get_json<T: serde::de::DeserializeOwned>(
    key: impl AsRef<[u8]>,
) -> FunctionResult<Option<T>> {
    let value = cache_scalar::get(key.as_ref())?;
    match value {
        Some(value) => serde_json::from_slice(&value)
            .map_err(|e| crate::Error::MessageError(format!("failed to deserialize json: {e}"))),
        None => Ok(None),
    }
}

/// Set a value in the cache with a time-to-live.
///
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::cache;
/// # use std::time::Duration;
///
/// # fn f() -> FunctionResult<()> {
/// cache::set(
///     "my_key",
///     b"hello".to_vec(),
///     Duration::from_secs(60),
/// )?;
/// # Ok(()) }
/// ```
pub fn set(key: impl AsRef<[u8]>, value: impl AsRef<[u8]>, ttl: Duration) -> FunctionResult<()> {
    cache_scalar::set(key.as_ref(), value.as_ref(), saturate_ttl(ttl)).map_err(Into::into)
}

/// Set a value in the cache with a time-to-live.
/// Serializes the value as JSON to store it in the cache.
///
/// On serialization failure, an error is returned.
///
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::cache;
/// # use std::time::Duration;
///
/// #[derive(serde::Serialize)]
/// struct MyStruct {
///    hello: String
/// }
///
/// # fn f() -> FunctionResult<()> {
/// cache::set_json(
///     "my_key",
///     MyStruct { hello: "hello".to_string() },
///     Duration::from_secs(60),
/// )?;
/// # Ok(()) }
/// ```
pub fn set_json(
    key: impl AsRef<[u8]>,
    value: impl serde::Serialize,
    ttl: Duration,
) -> FunctionResult<()> {
    let value = serde_json::to_vec(&value)
        .map_err(|e| crate::Error::MessageError(format!("failed to serialize json: {e}")))?;
    cache_scalar::set(key.as_ref(), &value, saturate_ttl(ttl)).map_err(Into::into)
}

fn saturate_ttl(ttl: Duration) -> u64 {
    ttl.as_millis().clamp(0, u64::MAX as u128) as u64
}
