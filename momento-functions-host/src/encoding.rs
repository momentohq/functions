//! Encoding and decoding of byte array payloads

use std::convert::Infallible;

/// Required to be implemented by encode error types.
pub trait EncodeError {}

impl EncodeError for Infallible {}

impl EncodeError for serde_json::Error {}

/// A payload which can be converted to a vector of bytes
pub trait Encode {
    /// The error type returned when encoding fails.
    type Error: EncodeError;
    /// Convert the payload to a vector of bytes
    fn try_serialize(self) -> Result<impl Into<Vec<u8>>, Self::Error>;
}

impl Encode for Vec<u8> {
    type Error = Infallible;
    fn try_serialize(self) -> Result<impl Into<Vec<u8>>, Self::Error> {
        Ok(self)
    }
}
impl Encode for &[u8] {
    type Error = Infallible;
    fn try_serialize(self) -> Result<impl Into<Vec<u8>>, Self::Error> {
        Ok(self)
    }
}
impl Encode for String {
    type Error = Infallible;
    fn try_serialize(self) -> Result<impl Into<Vec<u8>>, Self::Error> {
        Ok(self.into_bytes())
    }
}
impl Encode for &str {
    type Error = Infallible;
    fn try_serialize(self) -> Result<impl Into<Vec<u8>>, Self::Error> {
        Ok(self.as_bytes())
    }
}
impl Encode for Option<Vec<u8>> {
    type Error = Infallible;
    fn try_serialize(self) -> Result<impl Into<Vec<u8>>, Self::Error> {
        match self {
            Some(v) => Ok(v),
            None => Ok(Vec::new()),
        }
    }
}
impl Encode for () {
    type Error = Infallible;
    fn try_serialize(self) -> Result<impl Into<Vec<u8>>, Self::Error> {
        Ok([])
    }
}
impl Encode for serde_json::Value {
    type Error = serde_json::Error;
    fn try_serialize(self) -> Result<impl Into<Vec<u8>>, Self::Error> {
        serde_json::to_vec(&self)
    }
}

/// Required to be implemented by extract error types.
pub trait ExtractError {}

impl ExtractError for Infallible {}

impl ExtractError for serde_json::Error {}

/// Payload extractor for encodings
pub trait Extract: Sized {
    /// The error type returned when extraction fails.
    type Error: ExtractError;
    /// Convert from a payload to a value
    fn extract(payload: Vec<u8>) -> Result<Self, Self::Error>;
}

impl Extract for Vec<u8> {
    type Error = Infallible;
    fn extract(payload: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(payload)
    }
}

/// JSON encoding and decoding
pub struct Json<T>(pub T);
impl<T: serde::de::DeserializeOwned> Extract for Json<T> {
    type Error = serde_json::Error;
    fn extract(payload: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Json(serde_json::from_slice(&payload)?))
    }
}

impl<T: serde::Serialize> Encode for Json<T> {
    type Error = serde_json::Error;
    fn try_serialize(self) -> Result<impl Into<Vec<u8>>, Self::Error> {
        serde_json::to_vec(&self.0)
    }
}
