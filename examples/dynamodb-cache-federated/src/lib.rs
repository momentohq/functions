//! Caching proxy in front of DynamoDB, authenticated by federating into an IAM
//! role.
//!
//! The client changes only its endpoint (to this Function) and adds an
//! `x-momento-authorization` header. The AWS SDK still signs every request, so
//! the client uses placeholder credentials; this Function drops that signature
//! and re-signs as `ROLE_ARN` via `Authorization::Federated`.
//!
//! Env:
//! - `ROLE_ARN`: the IAM role Momento federates into to call DynamoDB.
//! - `DYNAMODB_REGION`: region of the DynamoDB endpoint, e.g. `us-west-2`.
//! - `KEY_ATTRIBUTES`: key attribute names per table, e.g. `orders=pk,sk;users=id`.
//!   Needed to invalidate after PutItem.

use example_dynamodb_cache_core::{
    KeyAttributes, dispatch, forward, require_header, without_headers,
};
use momento_functions_bytes::Data;
use momento_functions_guest_web::{WebEnvironment, WebError, WebResponse, WebResult, invoke};
use momento_functions_host_log::{LogDestination, configure_logs};
use momento_functions_http::{Authorization, IamRole};

/// The caller's placeholder signature and Momento credentials. The federated
/// signer adds its own `Authorization`, date, token and host.
const STRIPPED_HEADERS: &[&str] = &[
    "x-momento-authorization",
    "authorization",
    "x-amz-date",
    "x-amz-security-token",
    "x-amz-content-sha256",
    "host",
];

invoke!(dynamodb_cache_federated);
fn dynamodb_cache_federated(body: Data) -> WebResult<WebResponse> {
    let env = WebEnvironment::load();
    configure_logs([LogDestination::topic(env.function_name()).into()])?;

    let role_arn = require_env("ROLE_ARN")?;
    let region = require_env("DYNAMODB_REGION")?;
    let dynamodb_url = format!("https://dynamodb.{region}.amazonaws.com/");

    let action = match require_header("X-Amz-Target", env.headers()) {
        Ok(action) => action,
        Err(rejection) => return rejection.into_response(),
    };
    let headers = without_headers(env.headers(), STRIPPED_HEADERS);
    let key_attributes = KeyAttributes::from_env();

    dispatch(&action, body.into_bytes(), &key_attributes, |body| {
        let authorization = Authorization::Federated(IamRole {
            role_arn: role_arn.clone(),
            service: "dynamodb".to_string(),
        });
        forward(&dynamodb_url, &headers, body, authorization)
    })?
    .into_web_response()
}

/// A missing env var is this Function's misconfiguration, not the caller's
/// mistake, so it stays a 500.
fn require_env(name: &str) -> WebResult<String> {
    std::env::var(name).map_err(|_| {
        log::error!("Missing {name} env var");
        WebError::message(format!("Missing {name} env var"))
    })
}
