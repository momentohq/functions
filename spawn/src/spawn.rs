use momento_functions_bytes::Data;
use thiserror::Error;

use crate::wit::momento::spawn::spawn;

/// An error returned when spawning a function.
#[derive(Debug, Error)]
pub enum SpawnError {
    /// The function does not exist.
    #[error("function not found")]
    FunctionNotFound,
    /// The function failed to spawn.
    #[error("internal error")]
    InternalError,
    /// The function failed to spawn due to a limit, such as its concurrency limit.
    #[error("limit exceeded: {0}")]
    Limit(String),
}

impl From<spawn::SpawnError> for SpawnError {
    fn from(e: spawn::SpawnError) -> Self {
        match e {
            spawn::SpawnError::FunctionNotFound => SpawnError::FunctionNotFound,
            spawn::SpawnError::InternalError => SpawnError::InternalError,
            spawn::SpawnError::Limit(s) => SpawnError::Limit(s),
        }
    }
}

/// Spawn a Spawn Function by name with the given data.
///
/// Spawn Functions run in the same cache as the caller and do not return a
/// value; this returns once the function has been started.
///
/// # Arguments
/// * `function_name` - The name of the Spawn Function to run.
/// * `data` - The payload passed to the Spawn Function.
///
/// # Examples
/// ________
/// Spawn a function with a JSON payload:
/// ```rust,no_run
/// use momento_functions_spawn::spawn;
///
/// match spawn("put-s3", br#"{"key":"a"}"#.to_vec()) {
///     Ok(()) => {}
///     Err(e) => eprintln!("spawn failed: {e}"),
/// }
/// ```
pub fn spawn(function_name: impl Into<String>, data: impl Into<Data>) -> Result<(), SpawnError> {
    spawn::spawn_function(&function_name.into(), data.into().into()).map_err(Into::into)
}
