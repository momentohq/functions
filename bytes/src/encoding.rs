//! Encoding and decoding of byte array payloads

use std::convert::Infallible;

use crate::Data;

/// Required to be implemented by encode error types.
pub trait EncodeError: std::error::Error + 'static {}

impl EncodeError for Infallible {}

impl EncodeError for serde_json::Error {}

/// A payload which can be converted to a vector of bytes
pub trait Encode {
    /// The error type returned when encoding fails.
    type Error: EncodeError;
    /// Convert the payload to a vector of bytes
    fn try_serialize(self) -> Result<Data, Self::Error>;
}

impl Encode for Vec<u8> {
    type Error = Infallible;
    fn try_serialize(self) -> Result<Data, Self::Error> {
        Ok(self.into())
    }
}
impl Encode for &[u8] {
    type Error = Infallible;
    fn try_serialize(self) -> Result<Data, Self::Error> {
        Ok(self.to_vec().into())
    }
}
impl Encode for String {
    type Error = Infallible;
    fn try_serialize(self) -> Result<Data, Self::Error> {
        Ok(self.into_bytes().into())
    }
}
impl Encode for &str {
    type Error = Infallible;
    fn try_serialize(self) -> Result<Data, Self::Error> {
        Ok(self.as_bytes().to_vec().into())
    }
}
impl Encode for Option<Vec<u8>> {
    type Error = Infallible;
    fn try_serialize(self) -> Result<Data, Self::Error> {
        match self {
            Some(v) => Ok(v.into()),
            None => Ok(Vec::new().into()),
        }
    }
}
impl Encode for () {
    type Error = Infallible;
    fn try_serialize(self) -> Result<Data, Self::Error> {
        Ok(Vec::new().into())
    }
}
impl Encode for serde_json::Value {
    type Error = serde_json::Error;
    fn try_serialize(self) -> Result<Data, Self::Error> {
        serde_json::to_vec(&self).map(Into::into)
    }
}

/// Required to be implemented by extract error types.
pub trait ExtractError: std::error::Error + 'static {}

impl ExtractError for Infallible {}

impl ExtractError for serde_json::Error {}

/// Payload extractor for encodings
pub trait Extract: Sized {
    /// The error type returned when extraction fails.
    type Error: ExtractError;
    /// Convert from a payload to a value
    fn extract(payload: Data) -> Result<Self, Self::Error>;
}

impl Extract for Vec<u8> {
    type Error = Infallible;
    fn extract(payload: Data) -> Result<Self, Self::Error> {
        Ok(payload.into_bytes())
    }
}

impl Extract for Data {
    type Error = Infallible;

    fn extract(payload: Data) -> Result<Self, Self::Error> {
        Ok(payload)
    }
}

/// JSON encoding and decoding
pub struct Json<T>(pub T);
impl<T: serde::de::DeserializeOwned> Extract for Json<T> {
    type Error = serde_json::Error;
    fn extract(payload: Data) -> Result<Self, Self::Error> {
        Ok(Json(serde_json::from_slice(&payload.into_bytes())?))
    }
}

impl<T: serde::Serialize> Encode for Json<T> {
    type Error = serde_json::Error;
    fn try_serialize(self) -> Result<Data, Self::Error> {
        serde_json::to_vec(&self.0).map(Into::into)
    }
}
