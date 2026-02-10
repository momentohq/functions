use crate::wit::exports::momento::web_function::guest_function_web;
use momento_functions_bytes::encoding::Extract;

use crate::IntoWebResponse;
/// Create a handler that accepts a post payload and returns a response.
///
/// You can accept raw bytes (`Vec<u8>`) as input, or any type for which [Extract] is implemented.
/// If you choose to use an extracted type, this will automatically return a 400 error containing
/// the error details if the input bytes cannot be extracted into the specified input type.
/// If you would rather handle extraction errors yourself, you should accept raw bytes as input
/// and perform extraction yourself.
///
/// Your implementation function must return a value which implements the [IntoWebResponse] trait.
/// Implementations of this trait are provided for
/// - [crate::WebResponse]: A basic response representation and builder
/// - `WebResult<impl IntoWebResponse>`: Allows you to return results where errors will be converted
///   to 500 responses.
/// - [()]: Results in an empty 204.
/// - [String] and [&str]: Results in a 200 with the string body.
/// - `Vec<u8>` and `&[u8]`: Results in a 200 with the binary body.
/// - [momento_functions_bytes::encoding::Json]: Results in a 200 with the Json body, or a 500 if the Json could not be serialized.
///
/// You may also implement [IntoWebResponse] for your own types.
///
/// **Raw Bytes Input:**
/// ```rust
/// use std::error::Error;
///
/// use momento_functions_bytes::Data;
/// use momento_functions_guest_web::{invoke, WebResponse};
///
/// invoke!(ping);
/// fn ping(payload: Data) -> &'static str {
///     "pong"
/// }
/// ```
///
/// **Typed JSON Input:**
/// ```rust
/// use std::error::Error;
///
/// use momento_functions_guest_web::{invoke, WebResponse};
/// use momento_functions_bytes::encoding::Json;
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
/// invoke!(greet);
/// fn greet(Json(request): Json<Request>) -> Json<Response> {
///     Json(Response { message: format!("Hello, {}!", request.name) })
/// }
/// ```
#[macro_export]
macro_rules! invoke {
    ($post_handler: ident) => {
        struct WebFunction;
        momento_functions_guest_web::wit::export_web_function!(WebFunction);

        #[automatically_derived]
        impl momento_functions_guest_web::wit::exports::momento::web_function::guest_function_web::Guest for WebFunction {
            fn invoke(request: momento_functions_guest_web::wit::exports::momento::web_function::guest_function_web::Data) -> momento_functions_guest_web::wit::exports::momento::web_function::guest_function_web::Response {
                momento_functions_guest_web::invoke_template(request, $post_handler)
            }
        }
    };
}

/// An internal helper for the invoke! macro.
#[doc(hidden)]
#[allow(unused)]
pub fn invoke_template<TExtract, TResponse>(
    payload: guest_function_web::Data,
    handler: fn(request: TExtract) -> TResponse,
) -> guest_function_web::Response
where
    TExtract: Extract,
    TResponse: IntoWebResponse,
{
    let payload: momento_functions_bytes::Data = payload.into();
    let request = match TExtract::extract(payload) {
        Ok(request) => request,
        Err(error) => {
            return guest_function_web::Response {
                status: 400,
                headers: vec![],
                body: momento_functions_bytes::Data::from(
                    format!("Failed to parse request body: {error}")
                        .to_string()
                        .as_bytes()
                        .to_vec(),
                )
                .into(),
            };
        }
    };
    handler(request).response()
}
