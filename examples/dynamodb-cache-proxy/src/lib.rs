//! Caching proxy in front of DynamoDB, authenticated by the caller's own SigV4
//! signature.
//!
//! The client signs the request for `dynamodb.<region>.amazonaws.com` as usual.
//! A hook that runs *after* signing points the request at this Function and adds:
//! - `x-uri`: the real DynamoDB URL the request was signed for
//! - `x-momento-authorization`: the Momento API key
//!
//! This Function forwards the signed request unchanged (minus those two
//! headers and `Host`, which the HTTP client sets from `x-uri`). `x-uri` must
//! be a DynamoDB endpoint: responses are cached, so any other upstream would
//! let a caller plant arbitrary items in the cache.
//!
//! Env:
//! - `KEY_ATTRIBUTES`: key attribute names per table, e.g. `orders=pk,sk;users=id`.
//!   Needed to invalidate after PutItem.

use example_dynamodb_cache_core::{
    BadRequest, KeyAttributes, dispatch, forward, is_dynamodb_url, require_header, without_headers,
};
use momento_functions_bytes::Data;
use momento_functions_guest_web::{WebEnvironment, WebResponse, WebResult, invoke};
use momento_functions_host_log::{LogDestination, configure_logs};
use momento_functions_http::Authorization;

const STRIPPED_HEADERS: &[&str] = &["x-momento-authorization", "x-uri", "host"];

invoke!(dynamodb_cache_proxy);
fn dynamodb_cache_proxy(body: Data) -> WebResult<WebResponse> {
    let env = WebEnvironment::load();
    configure_logs([LogDestination::topic(env.function_name()).into()])?;

    let (action, dynamodb_url) = match (
        require_header("X-Amz-Target", env.headers()),
        require_header("x-uri", env.headers()),
    ) {
        (Ok(action), Ok(dynamodb_url)) => (action, dynamodb_url),
        (Err(rejection), _) | (_, Err(rejection)) => return rejection.into_response(),
    };
    if !is_dynamodb_url(&dynamodb_url) {
        return BadRequest::new(
            "InvalidUpstreamException",
            format!(
                "x-uri {dynamodb_url} is not https://dynamodb.<region>.amazonaws.com/ or https://<account>.ddb.<region>.amazonaws.com/"
            ),
        )
        .into_response();
    }
    let headers = without_headers(env.headers(), STRIPPED_HEADERS);
    let key_attributes = KeyAttributes::from_env();

    dispatch(&action, body.into_bytes(), &key_attributes, |body| {
        forward(&dynamodb_url, &headers, body, Authorization::None)
    })?
    .into_web_response()
}
