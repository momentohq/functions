use std::time::Duration;

use momento_functions_wit::host::momento::functions::cache_scalar;

use crate::FunctionResult;

pub fn get(key: impl AsRef<[u8]>) -> FunctionResult<Option<Vec<u8>>> {
    cache_scalar::get(key.as_ref()).map_err(Into::into)
}

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

pub fn set(key: impl AsRef<[u8]>, value: impl AsRef<[u8]>, ttl: Duration) -> FunctionResult<()> {
    cache_scalar::set(key.as_ref(), value.as_ref(), saturate_ttl(ttl)).map_err(Into::into)
}

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
