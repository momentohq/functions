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
/// #[derive(serde::Deserialize)]
/// struct Request {
///     name: String,
/// }
/// #[derive(serde::Serialize)]
/// struct Response {
///     message: String,
/// }
///
/// momento_functions::post!(greet, Request, Response);
/// fn greet(request: Request) -> FunctionResult<Response> {
///     Ok(Response { message: format!("Hello, {}!", request.name) })
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
            fn post(payload: Vec<u8>) -> Result<Vec<u8>, momento_functions_wit::function_web::momento::functions::types::InvocationError> {
                $post_handler(payload).map_err(Into::into)
            }
        }
    };

    ($post_handler: ident, $request: ident, $response: ident) => {
        use momento_functions_host::FunctionResult;
        struct WebFunction;
        momento_functions_wit::function_web::export_web_function!(WebFunction);

        #[automatically_derived]
        impl momento_functions_wit::function_web::exports::momento::functions::guest_function_web::Guest for WebFunction {
            fn post(payload: Vec<u8>) -> Result<Vec<u8>, momento_functions_wit::function_web::momento::functions::types::InvocationError> {
                let payload: $request = serde_json::from_slice(&payload)
                    .map_err(|e| momento_functions_wit::function_web::momento::functions::types::InvocationError::RequestError(format!("could not deserialize json: {e:?}")))?;
                let response: $response = $post_handler(payload)?;
                serde_json::to_vec(&response)
                    .map_err(|e| momento_functions_wit::function_web::momento::functions::types::InvocationError::RequestError(format!("could not serialize json: {e:?}")))
            }
        }
    }
}
