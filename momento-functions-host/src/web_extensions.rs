//! Host interface extensions for Web Functions
//!
//! These interfaces don't do anything on other kinds of Functions.

use momento_functions_wit::function_web::momento::functions::web_function_support;

/// Returns the headers for the web function, if any are present.
/// This consumes the headers and takes ownership of the value; multiple calls after will always
/// yield `an empty list.
pub fn headers() -> Vec<(String, String)> {
    web_function_support::headers()
        .into_iter()
        .map(|web_function_support::Header { name, value }| (name, value))
        .collect()
}

/// Returns the query parameters for the web function, if any are present.
/// This consumes the parameters and takes ownership of the value; multiple calls after will always
/// yield an empty list.
pub fn query_parameters() -> Vec<(String, String)> {
    web_function_support::query_parameters()
        .into_iter()
        .map(|web_function_support::QueryParameter { name, value }| (name, value))
        .collect()
}

/// Returns the metadata within the caller's token, if present.
/// This consumes the metadata and takes ownership of the value; multiple calls after will always
/// yield `None`.
pub fn token_metadata() -> Option<String> {
    web_function_support::token_metadata()
}

/// Returns the invocation ID of the currently invoked function. This may be helpful to you
/// if you want to connect a request ID to callers with the invocation that was used at that time.
pub fn invocation_id() -> String {
    web_function_support::invocation_id()
}
