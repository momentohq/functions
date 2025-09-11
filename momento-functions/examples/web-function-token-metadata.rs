use momento_functions_host::encoding::Json;

#[derive(serde::Serialize)]
struct Response {
    message: String,
}

momento_functions::post!(token_metadata);
/// Using the provided host support, this function returns the caller's provided metadata within the `token_id` field
/// of the Momento key. When calling `GenerateApiToken` or `GenerateDisposableToken`, you can provide
/// data serialized as a `String` in the `token_id` field.
fn token_metadata(_: Vec<u8>) -> Json<Response> {
    let maybe_token_metadata = momento_functions_host::web_extensions::token_metadata();
    if let Some(metadata) = maybe_token_metadata {
        Json(Response {
            message: format!("Token metadata provided: {}", metadata),
        })
    } else {
        Json(Response {
            message: "No metadata provided, try invoking with a Momento key that was generated with a populated 'token_id' field".to_string(),
        })
    }
}
