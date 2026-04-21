//! Returns the caller's provided metadata from the `token_id` field of the
//! Momento API key used to invoke this Function. When calling `GenerateApiToken`
//! or `GenerateDisposableToken`, you can pass a stringified payload in `token_id`
//! and retrieve it here.

use momento_functions_bytes::{Data, encoding::Json};
use momento_functions_guest_web::{WebEnvironment, invoke};

#[derive(serde::Serialize)]
struct Response {
    message: String,
}

invoke!(token_metadata);
fn token_metadata(_: Data) -> Json<Response> {
    match WebEnvironment::load().token_metadata() {
        Some(metadata) => Json(Response {
            message: format!("Token metadata provided: {metadata}"),
        }),
        None => Json(Response {
            message: "No metadata provided, try invoking with a Momento key that was generated with a populated 'token_id' field".to_string(),
        }),
    }
}
