//! Host interfaces for working with Momento Topics apis

use momento_functions_wit::host::momento::functions::topic;
use serde::Serialize;

use crate::{
    FunctionResult,
    encoding::{Json, Payload},
};

/// Publish a message to a topic in the cache this Function is running within.
///
/// Examples:
/// _________
/// String:
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::topics;
///
/// # fn f() -> FunctionResult<()> {
/// topics::publish("my_topic", "hello there")?;
/// # Ok(()) }
/// ```
/// ________
/// Bytes:
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::topics;
///
/// # fn f() -> FunctionResult<()> {
/// topics::publish("my_topic", b"hello there".to_vec())?;
/// # Ok(()) }
/// ```
/// ________
/// Json:
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::topics;
/// use momento_functions_host::encoding::Json;
///
/// #[derive(serde::Serialize)]
/// struct MyStruct {
///    hello: String
/// }
///
/// # fn f() -> FunctionResult<()> {
/// topics::publish("my_topic", Json(MyStruct{ hello: "hello".to_string() }))?;
/// # Ok(()) }
/// ```
pub fn publish<T: for<'a> PublishKind<'a>>(topic: impl AsRef<str>, value: T) -> FunctionResult<()> {
    match value.as_publish()? {
        Publish::Str(s) => topic::publish(topic.as_ref(), s),
        Publish::String(s) => topic::publish(topic.as_ref(), s.as_str()),
        Publish::Bytes(b) => topic::publish_bytes(
            topic.as_ref(),
            &b.try_serialize()?.map(Into::into).unwrap_or_default(),
        ),
    }
    .map_err(Into::into)
}

/// Bind a type to a kind of topic message
pub trait PublishKind<'a> {
    /// Type of payload to publish
    type Encoding: Payload;

    /// Convert this type into a publishable message
    fn as_publish(&'a self) -> FunctionResult<Publish<'a, Self::Encoding>>;
}
impl<'a> PublishKind<'a> for String {
    type Encoding = &'a [u8];

    fn as_publish(&'a self) -> FunctionResult<Publish<'a, Self::Encoding>> {
        Ok(Publish::Str(self))
    }
}
impl<'a> PublishKind<'a> for &str {
    type Encoding = &'a [u8];

    fn as_publish(&'a self) -> FunctionResult<Publish<'a, Self::Encoding>> {
        Ok(Publish::Str(self))
    }
}
impl<'a> PublishKind<'a> for &[u8] {
    type Encoding = &'a [u8];

    fn as_publish(&'a self) -> FunctionResult<Publish<'a, Self::Encoding>> {
        Ok(Publish::Bytes(self))
    }
}
impl<'a> PublishKind<'a> for Vec<u8> {
    type Encoding = &'a [u8];

    fn as_publish(&'a self) -> FunctionResult<Publish<'a, Self::Encoding>> {
        Ok(Publish::Bytes(self.as_slice()))
    }
}
impl<'a, T: Serialize> PublishKind<'a> for Json<T> {
    type Encoding = &'a [u8];

    fn as_publish(&'a self) -> FunctionResult<Publish<'a, Self::Encoding>> {
        serde_json::to_string(&self.0)
            .map_err(|e| crate::Error::MessageError(format!("failed to serialize json: {e}")))
            .map(Publish::String)
    }
}

/// Value to publish to a topic.
/// You can publish a string or bytes.
pub enum Publish<'a, P: Payload = &'a [u8]> {
    /// Publish a string to the topic
    Str(&'a str),
    /// Publish a string to the topic
    String(String),
    /// Publish encoded bytes to the topic
    Bytes(P),
}
