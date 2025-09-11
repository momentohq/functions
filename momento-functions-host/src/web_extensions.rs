//! Host interface extensions for Web Functions
//!
//! These interfaces don't do anything on other kinds of Functions.

use momento_functions_wit::function_web::momento::functions::web_function_support;

/// Returns the headers for the web function, if any are present.
/// This consumes the headers and takes ownership of the value; multiple calls after will always
/// yield `None`.
pub fn headers() -> Vec<(String, String)> {
    web_function_support::headers()
        .into_iter()
        .map(|web_function_support::Header { name, value }| (name, value))
        .collect()
}

/// Returns the metadata within the caller's token, if present.
/// This consumes the metadata and takes ownership of the value; multiple calls after will always
/// yield `None`.
pub fn token_metadata() -> Option<String> {
    web_function_support::token_metadata()
}
