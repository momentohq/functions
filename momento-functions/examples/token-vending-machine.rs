//! This example showcases how you can create a simple function that vends temporary, scoped permissions
//! without having to create something like a Lambda-backed API Gateway solution combined with some DNS route. However, this function
//! is still protected by auth required in order to invoke the function.
//!
//! You can modify this code as necessary for your needs, but this example will generate a scoped token that expires in 1 hour
//! that has the following permissions:
//! * Cache
//!     * Read/Write
//!     * All items within the cache
//!     * Only cache `foo`
//! * Topic
//!     * Read (subscribe)
//!     * Only cache `foo`
//!     * Topics beginning with prefix `notification-`
//!
//! You may also notice we are passing in a value for `token_id`. `token_id` is a bit of a
//! misnomer in the Momento protos, but consider it like a "secret value" you can pass through
//! to the generated JWT that you know is verified since is signed by Momento. It could be a
//! stringified JWT of your own service, a simple string, a nonce, you name it.
//!
//! For this example, we'll simply call it `"my very secret value"`.
use std::time::Duration;

use momento_functions::{WebResponse, WebResult};
use momento_functions_host::{
    encoding::Json,
    logging::LogDestination,
    token::{self, CachePermissions, Permissions, TopicPermissions},
};
use momento_functions_wit::host::momento::functions::token::TokenError;
use serde_json::json;

#[derive(serde::Serialize)]
struct Response {
    api_key: String,
    endpoint: String,
    valid_until: u64,
}

momento_functions::post!(greet);
fn greet(_payload: Vec<u8>) -> WebResult<WebResponse> {
    momento_functions_log::configure_logs([LogDestination::topic("token-vending-machine").into()])?;

    log::debug!("received request to generate a disposable token");
    let permissions = Permissions::new()
        .with_cache(
            CachePermissions::read_write()
                .with_cache("foo")
                .with_all_items(),
        )
        .with_topic(
            TopicPermissions::read_only()
                .with_cache("foo")
                .with_topic_prefix("notification-"),
        );

    // As documented above, consider this a secret value you want securely embedded and
    // signed in your generated JWT.
    let token_id = Some("my very secret value".to_string());

    match token::generate_disposable_token(
        Duration::from_hours(1).as_secs() as u32,
        permissions,
        token_id,
    ) {
        Ok(result) => {
            // For now we just convert the response into something we can serialize,
            // but if you want to inspect the result you may do so here.
            let body = Json(Response {
                api_key: result.api_key,
                endpoint: result.endpoint,
                valid_until: result.valid_until,
            });
            Ok(WebResponse::new()
                .with_status(200)
                .with_headers(vec![(
                    "Content-Type".to_string(),
                    "application/json".to_string(),
                )])
                .with_body(body)?)
        }
        Err(e) => {
            // Capture logging so we can help debug why requests are failing
            log::error!("Failed to generate token: {e:?}");
            let status = match &e {
                token::GenerateDisposableTokenError::TokenError(token_error) => match token_error {
                    TokenError::InvalidArgument(_) => 400,
                    TokenError::PermissionDenied(_) => 403,
                    TokenError::LimitExceeded(_) => 429,
                    TokenError::InternalError => 500,
                },
            };
            Ok(WebResponse::new().with_status(status).with_body(json!({
                "message": e.to_string(),
            }))?)
        }
    }
}
