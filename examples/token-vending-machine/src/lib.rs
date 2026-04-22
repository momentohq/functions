//! Vends temporary, scoped Momento disposable tokens. Useful for handing out
//! short-lived credentials without standing up a separate Lambda + API
//! Gateway, while still being protected by the auth required to invoke the
//! Function itself.
//!
//! This example issues a token that expires in 1 hour with:
//! * Cache: read/write on all items in cache `foo`
//! * Topic: subscribe-only on cache `foo`, topics prefixed with `notification-`
//!
//! `token_id` is a misnomer in the Momento protos — think of it as a secret
//! payload baked into the issued JWT (which Momento signs). It can be a
//! stringified JWT of your own service, a nonce, etc. Here we just use a
//! placeholder string.

use std::time::Duration;

use momento_functions_bytes::{Data, encoding::Json};
use momento_functions_guest_web::{WebEnvironment, WebResponse, WebResult, invoke};
use momento_functions_host_log::{LogDestination, configure_logs};
use momento_functions_token::{
    CachePermissions, Permissions, TopicPermissions, generate_disposable_token,
};
use serde_json::json;

#[derive(serde::Serialize)]
struct Response {
    api_key: String,
    endpoint: String,
    valid_until: u64,
}

invoke!(vend);
fn vend(_payload: Data) -> WebResult<WebResponse> {
    let env = WebEnvironment::load();
    configure_logs([LogDestination::topic(env.function_name()).into()])?;

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

    let token_id = Some("my very secret value".to_string());

    match generate_disposable_token(
        Duration::from_secs(60 * 60).as_secs() as u32,
        permissions,
        token_id,
    ) {
        Ok(result) => Ok(WebResponse::new()
            .with_status(200)
            .header("Content-Type", "application/json")
            .with_body(Json(Response {
                api_key: result.api_key,
                endpoint: result.endpoint,
                valid_until: result.valid_until,
            }))?),
        Err(e) => {
            log::error!("Failed to generate token: {e:?}");
            Ok(WebResponse::new().with_status(500).with_body(json!({
                "message": e.to_string(),
            }))?)
        }
    }
}
