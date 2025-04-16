use momento_functions_wit::host::momento::functions::types::InvocationError;

pub type FunctionResult<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invocation error: {0}")]
    InvocationError(#[from] InvocationError),

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
