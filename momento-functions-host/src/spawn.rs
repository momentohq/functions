use momento_functions_wit::host::momento::host::spawn::{self, SpawnError};

use crate::{FunctionResult, encoding::Encode};

/// Spawn a fire-and-forget Function.
///
/// ```rust
/// # use momento_functions_host::FunctionResult;
/// # use momento_functions_host::spawn;
///
/// # fn f() -> FunctionResult<()> {
/// spawn("my_function", b"a payload for my_function".as_slice())?;
/// # Ok(()) }
/// ```
pub fn spawn(function_name: impl AsRef<str>, payload: impl Encode) -> FunctionResult<()> {
    spawn::spawn_function(function_name.as_ref(), &payload.try_serialize()?.into())
        .map_err(Into::into)
}

impl From<SpawnError> for crate::Error {
    fn from(value: SpawnError) -> Self {
        match value {
            SpawnError::FunctionNotFound => {
                crate::Error::MessageError("function not found".to_string())
            }
            SpawnError::SpawnFailed(failed) => crate::Error::MessageError(failed),
            SpawnError::Limit(limit) => crate::Error::MessageError(limit),
        }
    }
}
