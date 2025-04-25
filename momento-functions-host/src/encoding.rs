//! Encoding and decoding of byte array payloads

use crate::FunctionResult;

/// A payload which can be converted to a vector of bytes
pub trait Payload {
    /// Convert the payload to a vector of bytes
    fn try_serialize(self) -> FunctionResult<Option<impl Into<Vec<u8>>>>;
}

impl Payload for Vec<u8> {
    fn try_serialize(self) -> FunctionResult<Option<impl Into<Vec<u8>>>> {
        Ok(Some(self))
    }
}
impl Payload for &[u8] {
    fn try_serialize(self) -> FunctionResult<Option<impl Into<Vec<u8>>>> {
        Ok(Some(self))
    }
}
impl Payload for String {
    fn try_serialize(self) -> FunctionResult<Option<impl Into<Vec<u8>>>> {
        Ok(Some(self.into_bytes()))
    }
}
impl Payload for &str {
    fn try_serialize(self) -> FunctionResult<Option<impl Into<Vec<u8>>>> {
        Ok(Some(self.as_bytes()))
    }
}
impl Payload for Option<Vec<u8>> {
    fn try_serialize(self) -> FunctionResult<Option<impl Into<Vec<u8>>>> {
        Ok(self)
    }
}
impl Payload for () {
    fn try_serialize(self) -> FunctionResult<Option<impl Into<Vec<u8>>>> {
        Ok(Option::<[u8; 0]>::None)
    }
}

/// Payload extractor for encodings
pub trait Extract: Sized {
    /// Convert from a payload to a value
    fn extract(payload: Vec<u8>) -> FunctionResult<Self>;
}

impl Extract for Vec<u8> {
    fn extract(payload: Vec<u8>) -> FunctionResult<Self> {
        Ok(payload)
    }
}

/// JSON encoding and decoding
pub struct Json<T>(pub T);
impl<T: serde::de::DeserializeOwned> Extract for Json<T> {
    fn extract(payload: Vec<u8>) -> FunctionResult<Self> {
        Ok(Json(serde_json::from_slice(&payload).map_err(|e| {
            crate::Error::MessageError(format!("failed to deserialize json: {e}"))
        })?))
    }
}
impl<T: serde::Serialize> Payload for Json<T> {
    fn try_serialize(self) -> FunctionResult<Option<impl Into<Vec<u8>>>> {
        let value = serde_json::to_vec(&self.0)
            .map_err(|e| crate::Error::MessageError(format!("failed to serialize json: {e}")))?;
        Ok(Some(value))
    }
}
