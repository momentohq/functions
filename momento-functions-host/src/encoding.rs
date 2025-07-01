//! Encoding and decoding of byte array payloads

use crate::FunctionResult;

/// A payload which can be converted to a vector of bytes
pub trait Encode {
    /// Convert the payload to a vector of bytes
    fn try_serialize(self) -> FunctionResult<impl Into<Vec<u8>>>;
}

impl Encode for Vec<u8> {
    fn try_serialize(self) -> FunctionResult<impl Into<Vec<u8>>> {
        Ok(self)
    }
}
impl Encode for &[u8] {
    fn try_serialize(self) -> FunctionResult<impl Into<Vec<u8>>> {
        Ok(self)
    }
}
impl Encode for String {
    fn try_serialize(self) -> FunctionResult<impl Into<Vec<u8>>> {
        Ok(self.into_bytes())
    }
}
impl Encode for &str {
    fn try_serialize(self) -> FunctionResult<impl Into<Vec<u8>>> {
        Ok(self.as_bytes())
    }
}
impl Encode for Option<Vec<u8>> {
    fn try_serialize(self) -> FunctionResult<impl Into<Vec<u8>>> {
        match self {
            Some(v) => Ok(v),
            None => Ok(Vec::new()),
        }
    }
}
impl Encode for () {
    fn try_serialize(self) -> FunctionResult<impl Into<Vec<u8>>> {
        Ok([])
    }
}
impl Encode for serde_json::Value {
    fn try_serialize(self) -> FunctionResult<impl Into<Vec<u8>>> {
        serde_json::to_vec(&self)
            .map_err(|e| crate::Error::MessageError(format!("failed to serialize json: {e:?}")))
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
impl<T: serde::Serialize> Encode for Json<T> {
    fn try_serialize(self) -> FunctionResult<impl Into<Vec<u8>>> {
        serde_json::to_vec(&self.0)
            .map_err(|e| crate::Error::MessageError(format!("failed to serialize json: {e}")))
    }
}
