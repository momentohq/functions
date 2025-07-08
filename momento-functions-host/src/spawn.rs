use momento_functions_wit::host::momento::host::spawn;

use crate::encoding::{Encode, EncodeError};

/// An error occurred while spawning a function.
#[derive(Debug, thiserror::Error)]
pub enum FunctionSpawnError<E: EncodeError> {
    /// An error occurred while calling the host interface function.
    #[error(transparent)]
    FunctionSpawnError(#[from] spawn::SpawnError),
    /// An error occurred while encoding the provided payload.
    #[error("Failed to encode payload")]
    EncodeFailed {
        /// The underlying encoding error.
        cause: E,
    },
}

/// Spawn a fire-and-forget Function.
///
/// ```rust
/// # use momento_functions_host::spawn::{self, FunctionSpawnError};
///
/// # fn f() -> Result<(), FunctionSpawnError<&'static str>> {
/// spawn("my_function", b"a payload for my_function".as_slice())?;
/// # Ok(()) }
/// ```
pub fn spawn<E: Encode>(
    function_name: impl AsRef<str>,
    payload: E,
) -> Result<(), FunctionSpawnError<E::Error>> {
    spawn::spawn_function(
        function_name.as_ref(),
        &payload
            .try_serialize()
            .map_err(|e| FunctionSpawnError::EncodeFailed { cause: e })?
            .into(),
    )
    .map_err(Into::into)
}
