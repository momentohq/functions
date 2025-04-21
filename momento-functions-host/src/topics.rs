//! Host interfaces for working with Momento Topics apis

use momento_functions_wit::host::momento::functions::topic;
use serde::Serialize;

use crate::FunctionResult;

/// Publish a message to a topic in the cache this Function is running within.
///
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::topics;
///
/// # fn f() -> FunctionResult<()> {
/// topics::publish("my_topic", "hello there")?;
/// # Ok(()) }
/// ```
pub fn publish(topic: impl AsRef<str>, value: impl AsRef<str>) -> FunctionResult<()> {
    topic::publish(topic.as_ref(), value.as_ref()).map_err(Into::into)
}

/// Publish a message to a topic in the cache this Function is running within.
///
/// Value is serialized as a JSON string.
///
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::topics;
///
/// #[derive(serde::Serialize)]
/// struct MyStruct {
///    hello: String
/// }
///
/// # fn f() -> FunctionResult<()> {
/// topics::publish_json("my_topic", MyStruct{ hello: "hello".to_string() })?;
/// # Ok(()) }
/// ```
pub fn publish_json(topic: impl AsRef<str>, value: impl Serialize) -> FunctionResult<()> {
    let value = serde_json::to_string(&value)
        .map_err(|e| crate::Error::MessageError(format!("failed to serialize json: {e}")))?;
    topic::publish(topic.as_ref(), &value).map_err(Into::into)
}
