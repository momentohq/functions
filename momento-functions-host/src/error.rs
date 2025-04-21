//! Common host interfaces types

use momento_functions_wit::host::momento::functions::types::InvocationError;

/// An alias for Result<T, Error> for convenience.
pub type FunctionResult<T> = std::result::Result<T, Error>;

/// An error during the execution of a Function.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// A low-level WIT error during the invocation of a Function.
    #[error("Invocation error: {0}")]
    InvocationError(#[from] InvocationError),

    /// A catch-all error with a message.
    #[error("{0}")]
    MessageError(String),
}

impl From<Error> for InvocationError {
    fn from(e: Error) -> Self {
        match e {
            Error::InvocationError(e) => e,
            Error::MessageError(msg) => InvocationError::RequestError(msg),
        }
    }
}

impl From<Error>
    for momento_functions_wit::function_web::momento::functions::types::InvocationError
{
    fn from(e: Error) -> Self {
        match e {
            Error::InvocationError(e) => momento_functions_wit::function_web::momento::functions::types::InvocationError::RequestError(e.to_string()),
            Error::MessageError(msg) => momento_functions_wit::function_web::momento::functions::types::InvocationError::RequestError(msg),
        }
    }
}

impl From<Error>
    for momento_functions_wit::function_spawn::momento::functions::types::InvocationError
{
    fn from(e: Error) -> Self {
        match e {
            Error::InvocationError(e) => momento_functions_wit::function_spawn::momento::functions::types::InvocationError::RequestError(e.to_string()),
            Error::MessageError(msg) => momento_functions_wit::function_spawn::momento::functions::types::InvocationError::RequestError(msg),
        }
    }
}
