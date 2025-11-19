use momento_functions::{WebError, WebResponse, WebResult};
use momento_functions_host::{
    aws::ddb::KeyValue,
    cache,
    encoding::{Extract, Json},
    http,
    logging::LogDestination,
    web_extensions::headers,
};
use std::{collections::HashMap, time::Duration};

momento_functions::post!(accelerate_get_item);
fn accelerate_get_item(body: Vec<u8>) -> WebResult<WebResponse> {
    let headers = headers();
    setup_logging(&headers)?;

    // Extract the required headers

    // The target header comes from the AWS SDK and is the api call being made.
    let action = match require_header("X-Amz-Target", &headers) {
        Ok(value) => value,
        Err(value) => return value,
    };
    // The x-uri header is the custom header we added to the request _after it was signed_,
    // as we changed the request's target uri to _this Function_.
    let proxy_uri = match require_header("x-uri", &headers) {
        Ok(value) => value,
        Err(value) => return value,
    };

    let http::Response {
        status,
        headers,
        body,
    } = match action.as_str() {
        "DynamoDB_20120810.GetItem" => handle_get_item(body, headers, &proxy_uri)?,
        other => {
            // We could invalidate the cache on a putitem to the same key, but that's omitted here for brevity.
            handle_all_other_ddb_calls(other, body, headers, &proxy_uri)?
        }
    };

    Ok(WebResponse::new()
        .with_status(status)
        .with_headers(headers)
        .with_body(body)?)
}

// ------------------------------------------------------
// | Handlers for DynamoDB API calls
// ------------------------------------------------------

fn handle_get_item(
    body: Vec<u8>,
    headers: Vec<(String, String)>,
    proxy_uri: &str,
) -> WebResult<http::Response> {
    #[derive(serde::Deserialize, serde::Serialize, Debug)]
    struct GetItemRequest {
        #[serde(rename = "TableName")]
        table_name: String,
        #[serde(rename = "Key")]
        key: HashMap<String, KeyValue>,
    }

    let Json(request) = Json::<GetItemRequest>::extract(body.clone())?;
    log::info!("GetItem {request:?}");
    let cache_key: String = serde_json::to_string(&request)?;

    Ok(match cache::get::<Json<http::Response>>(&cache_key)? {
        Some(Json(hit)) => {
            log::info!("Cache hit for {cache_key}");
            hit
        }
        None => {
            log::info!("Cache miss for {cache_key} -> {proxy_uri}");
            let response = http::post(proxy_uri, headers, body)?;
            cache::set(&cache_key, Json(&response), Duration::from_secs(60))?;
            response
        }
    })
}

fn handle_all_other_ddb_calls(
    action: &str,
    body: Vec<u8>,
    headers: Vec<(String, String)>,
    proxy_uri: &str,
) -> WebResult<http::Response> {
    log::info!("other action: {action} -> {proxy_uri}");
    Ok(http::post(proxy_uri, headers, body)?)
}

// ------------------------------------------------------
// | Utility functions for convenience
// ------------------------------------------------------

fn require_header(
    header: &str,
    headers: &[(String, String)],
) -> Result<String, WebResult<WebResponse>> {
    let action = match headers.iter().find_map(|(name, value)| {
        if name.eq_ignore_ascii_case(header) {
            Some(value)
        } else {
            None
        }
    }) {
        Some(action) => action,
        None => {
            log::error!("Missing {header} header");
            return Err(WebResponse::new()
                .with_status(400)
                .with_body(format!("Missing {header} header"))
                .map_err(WebError::from));
        }
    };
    Ok(action.to_string())
}

fn setup_logging(_headers: &[(String, String)]) -> WebResult<()> {
    momento_functions_log::configure_logs([LogDestination::topic("ddb-accelerator").into()])?;
    Ok(())
}
