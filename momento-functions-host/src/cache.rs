//! Host interfaces for working with Momento Cache apis

use std::time::Duration;

use momento_functions_wit::host::momento::functions::cache_scalar;

use crate::{
    FunctionResult,
    encoding::{Extract, Payload},
};

/// Get a value from the cache.
///
/// Examples:
/// ________
/// Bytes:
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::cache;
///
/// # fn f() -> FunctionResult<()> {
/// let value: Option<Vec<u8>> = cache::get("my_key")?;
/// # Ok(()) }
/// ```
/// ________
/// Json:
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::cache;
/// use momento_functions_host::encoding::Json;
///
/// #[derive(serde::Deserialize)]
/// struct MyStruct {
///   message: String
/// }
///
/// # fn f() -> FunctionResult<()> {
/// let value: Option<Json<MyStruct>> = cache::get("my_key")?;
/// # Ok(()) }
/// ```
pub fn get<T: Extract>(key: impl AsRef<[u8]>) -> FunctionResult<Option<T>> {
    match cache_scalar::get(key.as_ref()).map_err(crate::Error::from)? {
        Some(v) => T::extract(v).map(Some),
        None => Ok(None),
    }
}

/// Set a value in the cache with a time-to-live.
///
/// Examples:
/// ________
/// Bytes:
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
/// ________
/// Json:
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::cache;
/// # use std::time::Duration;
/// use momento_functions_host::encoding::Json;
///
/// #[derive(serde::Serialize)]
/// struct MyStruct {
///    hello: String
/// }
///
/// # fn f() -> FunctionResult<()> {
/// cache::set(
///     "my_key",
///     Json(MyStruct { hello: "hello".to_string() }),
///     Duration::from_secs(60),
/// )?;
/// # Ok(()) }
/// ```
pub fn set(key: impl AsRef<[u8]>, value: impl Payload, ttl: Duration) -> FunctionResult<()> {
    cache_scalar::set(
        key.as_ref(),
        &value.try_serialize()?.map(Into::into).unwrap_or_default(),
        saturate_ttl(ttl),
    )
    .map_err(Into::into)
}

fn saturate_ttl(ttl: Duration) -> u64 {
    ttl.as_millis().clamp(0, u64::MAX as u128) as u64
}
