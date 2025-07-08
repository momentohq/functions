//! Host interfaces for working with Momento Topics apis

use momento_functions_wit::host::momento::functions::topic;
use serde::Serialize;

use crate::encoding::{Encode, Json};

/// An error occurred while publihsing to a topic.
#[derive(Debug, thiserror::Error)]
pub enum PublishError<E: Encode> {
    /// An error occurred while encoding the provided message.
    #[error("Failed to encode message")]
    EncodeFailed {
        /// The underlying encoding error.
        cause: E::Error,
    },
    /// An error occurred while calling the host publish function.
    #[error(transparent)]
    PublishError(#[from] topic::Error),
}

/// Publish a message to a topic in the cache this Function is running within.
///
/// Examples:
/// _________
/// String:
/// ```rust
/// # use momento_functions_host::topics;///
/// #
/// use momento_functions_host::topics::PublishError;
///
/// fn f() -> Result<(), PublishError<&'static str>> {
/// topics::publish("my_topic", "hello there")?;
/// # Ok(()) }
/// ```
/// ________
/// Bytes:
/// ```rust
/// # use momento_functions_host::topics;///
/// #
/// use momento_functions_host::topics::PublishError;
///
/// fn f() -> Result<(), PublishError<&'static str>> {
/// topics::publish("my_topic", b"hello there".to_vec())?;
/// # Ok(()) }
/// ```
/// ________
/// Json:
/// ```rust
/// # use momento_functions_host::topics;
/// use momento_functions_host::encoding::Json;///
///
/// use momento_functions_host::topics::PublishError;
///
/// #[derive(serde::Serialize)]
/// struct MyStruct {
///    hello: String
/// }
///
/// # fn f() -> Result<(), PublishError<Json<MyStruct>>> {
/// topics::publish("my_topic", Json(MyStruct{ hello: "hello".to_string() }))?;
/// # Ok(()) }
/// ```
pub fn publish<T: PublishKind>(
    topic: impl AsRef<str>,
    value: T,
) -> Result<(), PublishError<<T as PublishKind>::Encoding>> {
    match value
        .as_publish()
        .map_err(|e| PublishError::EncodeFailed { cause: e })?
    {
        Publish::Str(s) => topic::publish(topic.as_ref(), s),
        Publish::String(s) => topic::publish(topic.as_ref(), s.as_str()),
        Publish::Bytes(b) => topic::publish_bytes(
            topic.as_ref(),
            &b.try_serialize()
                .map_err(|e| PublishError::EncodeFailed { cause: e })?
                .into(),
        ),
    }
    .map_err(Into::into)
}

/// Bind a type to a kind of topic message
pub trait PublishKind {
    /// Type of payload to publish
    type Encoding: Encode;

    /// Convert this type into a publishable message
    fn as_publish(&self) -> Result<Publish<Self::Encoding>, <Self::Encoding as Encode>::Error>;
}
impl PublishKind for String {
    type Encoding = String;

    fn as_publish(&self) -> Result<Publish<Self::Encoding>, <Self::Encoding as Encode>::Error> {
        Ok(Publish::Str(self))
    }
}
impl<'a> PublishKind for &'a str {
    type Encoding = &'a str;

    fn as_publish(&self) -> Result<Publish<'a, Self::Encoding>, <Self::Encoding as Encode>::Error> {
        Ok(Publish::Str(self))
    }
}
impl<'a> PublishKind for &'a [u8] {
    type Encoding = &'a [u8];

    fn as_publish(&self) -> Result<Publish<'a, Self::Encoding>, <Self::Encoding as Encode>::Error> {
        Ok(Publish::Bytes(self))
    }
}
impl PublishKind for Vec<u8> {
    type Encoding = Vec<u8>;

    fn as_publish(&self) -> Result<Publish<Self::Encoding>, <Self::Encoding as Encode>::Error> {
        Ok(Publish::Bytes(self.clone()))
    }
}
impl<T: Serialize> PublishKind for Json<T> {
    type Encoding = Json<T>;

    fn as_publish(&self) -> Result<Publish<Self::Encoding>, <Self::Encoding as Encode>::Error> {
        serde_json::to_string(&self.0).map(Publish::String)
    }
}

/// Value to publish to a topic.
/// You can publish a string or bytes.
pub enum Publish<'a, P: Encode = &'a [u8]> {
    /// Publish a string to the topic
    Str(&'a str),
    /// Publish a string to the topic
    String(String),
    /// Publish encoded bytes to the topic
    Bytes(P),
}
