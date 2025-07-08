use momento_functions_host::encoding::{Encode, Extract};
use momento_functions_wit::function_web::exports::momento::functions::guest_function_web;

use crate::response::WebResponse;

/// Create a handler that accepts a post payload and returns a response.
/// Unlike [crate::post], this macro requires that your implementation accept a raw [Vec<u8>] as input
/// and return a [guest_function_web::Response] as output.
///
/// You may still make use of `Extract` and
/// `Encode` within your implementation, but this macro requires that you handle payload extraction
/// and any errors which arise within your implementation.
///
/// This affords you more control over what to do when an error occurs.
///
/// ```rust
/// use momento_functions_host::encoding::{Encode, Extract, Json};
/// use momento_functions_wit::function_web::exports::momento::functions::guest_function_web;
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
/// momento_functions::post_raw!(greet);
/// fn greet(payload: Vec<u8>) -> guest_function_web::Response {
///     let Json(request) = match Json::<Request>::extract(payload) {
///         Ok(user) => user,
///         Err(extraction_error) => return guest_function_web::Response {
///             status: 400,
///             headers: vec![],
///             body: format!("Invalid input! {extraction_error:?}").as_bytes().to_vec(),
///         }
///     };
///
///     let response_body: Vec<u8> = match Json(Response {
///         message: format!("Hello, {}!", request.name)
///     }).try_serialize() {
///         Ok(body) => body,
///         Err(_encoding_error) => return guest_function_web::Response {
///             status: 500,
///             headers: vec![],
///             body: "Internal error".as_bytes().to_vec()
///         }
///     };
///
///     guest_function_web::Response {
///         status: 200,
///         headers: vec![("name".to_string(), request.name).into()],
///         body: response_body
///     }
/// }
/// ```
#[macro_export]
macro_rules! post_raw {
    ($post_handler: ident) => {
        struct WebFunction;
        momento_functions_wit::__export_web_function_impl!(WebFunction);

        #[automatically_derived]
        impl momento_functions_wit::function_web::exports::momento::functions::guest_function_web::Guest for WebFunction {
            fn post(payload: Vec<u8>) -> momento_functions_wit::function_web::exports::momento::functions::guest_function_web::Response {
                $post_handler(payload)
            }
        }
    };
}

/// Create a handler that accepts a post payload and returns a response.
///
/// You can use raw bytes, or json-marshalled types.
///
/// This will automatically return a 400 error containing the error details if the input bytes cannot be marshalled into the specified type.
///
/// This will automatically return a 500 error containing the error details if an error is returned from your implementation.
///
/// If you want more control over input parsing and/or want to enforce that errors must be handled within
/// your implementation, consider using [crate::post_raw].
///
/// **Raw:**
/// ```rust
/// use std::error::Error;
///
/// momento_functions::post!(ping);
/// fn ping(payload: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
///     Ok(b"pong".to_vec())
/// }
/// ```
///
/// **Typed JSON:**
/// ```rust
/// use std::error::Error;
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
/// fn greet(Json(request): Json<Request>) -> Result<Json<Response>, Box<dyn Error>> {
///     Ok(Json(Response { message: format!("Hello, {}!", request.name) }))
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
    handler: fn(request: TExtract) -> Result<TResponse, Box<dyn std::error::Error>>,
) -> guest_function_web::Response
where
    TExtract: Extract,
    TResponse: WebResponse,
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
    let mut response = match handler(request) {
        Ok(response) => response,
        Err(error) => return error_to_500_response(error),
    };
    let status = response.get_status_code();
    let headers: Vec<(String, String)> = response.take_headers();
    let body: Vec<u8> = match response.take_payload().try_serialize() {
        Ok(body) => body,
        Err(error) => return error_to_500_response(error.into()),
    }
    .into();
    guest_function_web::Response {
        status,
        headers: headers.into_iter().map(Into::into).collect(),
        body,
    }
}

#[inline]
fn error_to_500_response(error: Box<dyn std::error::Error>) -> guest_function_web::Response {
    guest_function_web::Response {
        status: 500,
        headers: vec![],
        body: format!("An error occurred during function invocation: {error}")
            .to_string()
            .as_bytes()
            .to_vec(),
    }
}
