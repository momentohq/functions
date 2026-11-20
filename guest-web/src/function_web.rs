use crate::wit::exports::momento::web_function::guest_function_web;
use momento_functions_bytes::encoding::Extract;

use crate::IntoWebResponse;
/// Create a handler that accepts a post payload and returns a response.
///
/// You can accept the request body as [momento_functions_bytes::Data], or as any type for which
/// [Extract] is implemented, such as `Vec<u8>`, `String`, or
/// [momento_functions_bytes::encoding::Json].
///
/// The request body may still be arriving when your function starts. Accepting `Data` lets you
/// read it through `std::io::Read` as it arrives. Any other type waits for the whole body first.
///
/// If the body cannot be extracted into your input type, this automatically returns an error
/// response: 408 if the body did not finish arriving before the call's deadline, otherwise 400
/// with the error details. If you would rather handle these errors yourself, accept `Data` and
/// read or extract it yourself.
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
            let (status, message) = if TExtract::timed_out(&error) {
                (408, "Timed out reading request body".to_string())
            } else {
                (400, format!("Failed to parse request body: {error}"))
            };
            return guest_function_web::Response {
                status,
                headers: vec![],
                body: momento_functions_bytes::Data::from(message).into(),
            };
        }
    };
    handler(request).response()
}
