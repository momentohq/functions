//! Caches DynamoDB GetItem responses by acting as a proxy in front of DynamoDB.
//! Other DynamoDB API calls are passed straight through. The AWS SDK signs the
//! request for `dynamodb.<region>.amazonaws.com`; the caller rewrites the URL
//! to point at this Function and stashes the original target in the `x-uri`
//! header (which is added *after* signing).

use std::{collections::HashMap, time::Duration};

use momento_functions_bytes::{
    Data,
    encoding::{Extract, Json},
};
use momento_functions_cache as cache;
use momento_functions_guest_web::{WebEnvironment, WebError, WebResponse, WebResult, invoke};
use momento_functions_host_log::{LogDestination, configure_logs};
use momento_functions_http::{Request as HttpRequest, invoke as http_invoke};
use serde::{Deserialize, Serialize};

invoke!(accelerate_get_item);
fn accelerate_get_item(body: Data) -> WebResult<WebResponse> {
    setup_logging()?;

    let env = WebEnvironment::load();
    let headers = env.headers();
    let body_bytes = body.into_bytes();

    let action = require_header("X-Amz-Target", headers)?;
    let proxy_uri = require_header("x-uri", headers)?;

    let CachedResponse {
        status,
        headers: response_headers,
        body: response_body,
    } = match action.as_str() {
        "DynamoDB_20120810.GetItem" => handle_get_item(body_bytes, headers, &proxy_uri)?,
        other => handle_passthrough(other, body_bytes, headers, &proxy_uri)?,
    };

    let mut response = WebResponse::new().with_status(status);
    for (k, v) in response_headers {
        response = response.header(k, v);
    }
    Ok(response.with_body(response_body)?)
}

#[derive(Serialize, Deserialize)]
struct CachedResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

fn handle_get_item(
    body: Vec<u8>,
    headers: &HashMap<String, String>,
    proxy_uri: &str,
) -> WebResult<CachedResponse> {
    #[derive(Deserialize, Serialize, Debug)]
    struct GetItemRequest {
        #[serde(rename = "TableName")]
        table_name: String,
        #[serde(rename = "Key")]
        key: serde_json::Value,
    }

    let Json(request) = Json::<GetItemRequest>::extract(Data::from(body.clone()))?;
    log::info!("GetItem {request:?}");
    let cache_key: String = serde_json::to_string(&request)?;

    if let Some(Json(hit)) = cache::get::<Json<CachedResponse>>(cache_key.as_bytes().to_vec())? {
        log::info!("Cache hit for {cache_key}");
        return Ok(hit);
    }

    log::info!("Cache miss for {cache_key} -> {proxy_uri}");
    let response = forward(proxy_uri, headers, body)?;
    cache::set(
        cache_key.as_bytes().to_vec(),
        Json(&response),
        Duration::from_secs(60),
    )?;
    Ok(response)
}

fn handle_passthrough(
    action: &str,
    body: Vec<u8>,
    headers: &HashMap<String, String>,
    proxy_uri: &str,
) -> WebResult<CachedResponse> {
    log::info!("other action: {action} -> {proxy_uri}");
    forward(proxy_uri, headers, body)
}

fn forward(
    proxy_uri: &str,
    headers: &HashMap<String, String>,
    body: Vec<u8>,
) -> WebResult<CachedResponse> {
    let mut request = HttpRequest::new(proxy_uri, "POST").with_body(body);
    for (name, value) in headers {
        request = request.with_header(name.clone(), value.clone());
    }
    let response = http_invoke(request)?;
    Ok(CachedResponse {
        status: response.status,
        headers: response.headers,
        body: response.body.into_bytes(),
    })
}

fn require_header(header: &str, headers: &HashMap<String, String>) -> WebResult<String> {
    headers
        .iter()
        .find_map(|(name, value)| name.eq_ignore_ascii_case(header).then(|| value.to_string()))
        .ok_or_else(|| {
            log::error!("Missing {header} header");
            WebError::message(format!("Missing {header} header"))
        })
}

fn setup_logging() -> WebResult<()> {
    let env = WebEnvironment::load();
    configure_logs([LogDestination::topic(env.function_name()).into()])?;
    Ok(())
}
