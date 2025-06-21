use momento_functions_host::{
    FunctionResult,
    encoding::{Encode, Extract},
};
use momento_functions_wit::function_web::exports::momento::functions::guest_function_web;

use crate::response::WebResponse;

/// Create a handler that accepts a post payload and returns a response.
///
/// You can use raw bytes, or json-marshalled types.
///
/// **Raw:**
/// ```rust
/// momento_functions::post!(ping);
/// fn ping(payload: Vec<u8>) -> FunctionResult<Vec<u8>> {
///     Ok(b"pong".to_vec())
/// }
/// ```
///
/// **Typed JSON:**
/// ```rust
/// use momento_functions_host::encoding::Json;
///
/// #[derive(serde::Deserialize)]
/// struct Request {
///     name: String,
/// }
/// #[derive(serde::Serialize)]
/// struct Response {
///     message: String,
/// }
///
/// momento_functions::post!(greet);
/// fn greet(Json(request): Json<Request>) -> FunctionResult<Json<Response>> {
///     Ok(Json(Response { message: format!("Hello, {}!", request.name) }))
/// }
/// ```
#[macro_export]
macro_rules! post {
    ($post_handler: ident) => {
        use momento_functions_host::FunctionResult;
        struct WebFunction;
        momento_functions_wit::__export_web_function_impl!(WebFunction);

        #[automatically_derived]
        impl momento_functions_wit::function_web::exports::momento::functions::guest_function_web::Guest for WebFunction {
            fn post(payload: Vec<u8>) -> Result<momento_functions_wit::function_web::exports::momento::functions::guest_function_web::Response, momento_functions_wit::function_web::momento::functions::types::InvocationError> {
                momento_functions::post_template(payload, $post_handler)
            }
        }
    };
}

/// An internal helper for the post! macro.
#[doc(hidden)]
pub fn post_template<TExtract, TResponse>(
    payload: Vec<u8>,
    handler: fn(request: TExtract) -> FunctionResult<TResponse>,
) -> Result<
    guest_function_web::Response,
    momento_functions_wit::function_web::momento::functions::types::InvocationError,
>
where
    TExtract: Extract,
    TResponse: WebResponse,
{
    let request = TExtract::extract(payload)?;
    let mut response = handler(request)?;

    let status = response.get_status_code();
    let headers: Vec<(String, String)> = response.take_headers();
    let body: Vec<u8> = response.take_payload().try_serialize()?.into();
    Ok(guest_function_web::Response {
        status,
        headers: headers.into_iter().map(Into::into).collect(),
        body,
    })
}
