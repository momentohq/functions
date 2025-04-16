use momento_functions_wit::host::momento::functions::topic;

use crate::FunctionResult;

/// Publish a message to a topic in the cache this Function is running within.
pub fn publish(topic: impl AsRef<str>, value: impl AsRef<str>) -> FunctionResult<()> {
    topic::publish(topic.as_ref(), value.as_ref()).map_err(Into::into)
}
