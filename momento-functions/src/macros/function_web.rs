use momento_functions_host::encoding::Extract;
use momento_functions_wit::function_web::exports::momento::functions::guest_function_web;

use crate::response::IntoWebResponse;
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
/// - [WebResponse]: A basic response representation and builder
/// - `WebResult<impl IntoWebResponse>`: Allows you to return results where errors will be converted
///   to 500 responses.
/// - [()]: Results in an empty 204.
/// - [String] and [&str]: Results in a 200 with the string body.
/// - `Vec<u8>` and `&[u8]`: Results in a 200 with the binary body.
/// - [Json]: Results in a 200 with the Json body, or a 500 if the Json could not be serialized.
///
/// You may also implement [IntoWebResponse] for your own types.
///
/// **Raw Bytes Input:**
/// ```rust
/// use std::error::Error;///
///
/// use momento_functions::WebResponse;
///
/// momento_functions::post!(ping);
/// fn ping(payload: Vec<u8>) -> &'static str {
///     "pong"
/// }
/// ```
///
/// **Typed JSON Input:**
/// ```rust
/// use std::error::Error;
/// use momento_functions::WebResponse;
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
/// fn greet(Json(request): Json<Request>) -> Json<Response> {
///     Json(Response { message: format!("Hello, {}!", request.name) })
/// }
/// ```
#[macro_export]
macro_rules! post {
    ($post_handler: ident) => {
        struct WebFunction;
        momento_functions_wit::__export_web_function_impl!(WebFunction);

        #[automatically_derived]
        impl momento_functions_wit::function_web::exports::momento::functions::guest_function_web::Guest for WebFunction {
            fn post(payload: Vec<u8>) -> momento_functions_wit::function_web::exports::momento::functions::guest_function_web::Response {
                momento_functions::post_template(payload, $post_handler)
            }
        }
    };
}

/// An internal helper for the post! macro.
#[doc(hidden)]
pub fn post_template<TExtract, TResponse>(
    payload: Vec<u8>,
    handler: fn(request: TExtract) -> TResponse,
) -> guest_function_web::Response
where
    TExtract: Extract,
    TResponse: IntoWebResponse,
{
    let request = match TExtract::extract(payload) {
        Ok(request) => request,
        Err(error) => {
            return guest_function_web::Response {
                status: 400,
                headers: vec![],
                body: format!("Failed to parse request body: {error}")
                    .to_string()
                    .as_bytes()
                    .to_vec(),
            };
        }
    };
    handler(request).response()
}
