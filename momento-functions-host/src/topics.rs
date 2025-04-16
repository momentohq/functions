use momento_functions_wit::host::momento::functions::topic;

use crate::FunctionResult;

/// Publish a message to a topic in the cache this Function is running within.
pub fn publish(topic: &str, value: &str) -> FunctionResult<()> {
    topic::publish(topic, value).map_err(Into::into)
}
