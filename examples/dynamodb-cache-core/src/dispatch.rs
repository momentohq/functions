use std::time::Duration;

use momento_functions_cache::{self as cache, ConditionalSetResult, SetIfCondition};
use momento_functions_guest_web::WebResult;
use serde_json::Value;

use crate::{
    CachedResponse, KeyAttributes, WriteAction, cacheable_get_item, items_to_invalidate, tombstone,
};

const GET_ITEM_TTL: Duration = Duration::from_secs(60);

/// PartiQL calls can write, but their keys are inside SQL text.
const PARTIQL_ACTIONS: &[&str] = &[
    "DynamoDB_20120810.ExecuteStatement",
    "DynamoDB_20120810.BatchExecuteStatement",
    "DynamoDB_20120810.ExecuteTransaction",
];

/// Route one DynamoDB call by its `X-Amz-Target` action.
///
/// - GetItem: served from cache when possible; 2xx responses cached for 60s.
/// - [`WriteAction`]s: forwarded, then the written items' cache entries are
///   invalidated unless DynamoDB clearly rejected the write.
/// - Everything else: forwarded unchanged.
///
/// `forward` sends a body to DynamoDB; it is the only part that differs
/// between the proxy and federated Functions.
pub fn dispatch(
    target: &str,
    body: Vec<u8>,
    key_attributes: &KeyAttributes,
    forward: impl Fn(Vec<u8>) -> WebResult<CachedResponse>,
) -> WebResult<CachedResponse> {
    // A body DynamoDB can't parse either is DynamoDB's to reject.
    let Ok(request) = serde_json::from_slice::<Value>(&body) else {
        log::warn!("{target}: unparseable body; passing through");
        return forward(body);
    };

    if target == "DynamoDB_20120810.GetItem" {
        return get_item(&request, body, forward);
    }
    if let Some(action) = WriteAction::from_target(target) {
        return write(action, &request, body, key_attributes, forward);
    }
    if PARTIQL_ACTIONS.contains(&target) {
        log::warn!("{target}: PartiQL writes are not invalidated; cached reads may be stale");
    } else {
        log::info!("{target}: pass through");
    }
    forward(body)
}

fn get_item(
    request: &Value,
    body: Vec<u8>,
    forward: impl Fn(Vec<u8>) -> WebResult<CachedResponse>,
) -> WebResult<CachedResponse> {
    let Some(item) = cacheable_get_item(request) else {
        log::info!("GetItem: not cacheable; pass through");
        return forward(body);
    };
    let key = item.cache_key();

    match cache::get::<Vec<u8>>(key.clone()) {
        Ok(Some(hit)) if tombstone::is_tombstone(&hit) => {
            log::info!("GetItem: recently written; reading through without caching");
            return forward(body);
        }
        Ok(Some(hit)) => match serde_json::from_slice::<CachedResponse>(&hit) {
            Ok(response) => {
                log::info!("GetItem: hit {}", String::from_utf8_lossy(&key));
                return Ok(response);
            }
            Err(e) => log::warn!("GetItem: corrupt cache entry, treating as miss: {e}"),
        },
        Ok(None) => {}
        Err(e) => log::warn!("GetItem: cache get failed, treating as miss: {e}"),
    }

    log::info!("GetItem: miss {}", String::from_utf8_lossy(&key));
    let response = forward(body)?;
    // Errors and throttles must reach the SDK every time so it can retry.
    if response.is_success() {
        fill(key, &response);
    }
    Ok(response)
}

/// Cache a GetItem response, but only into an absent key: a write that landed
/// while this read was in flight has left a tombstone there, and this response
/// may predate it.
fn fill(key: Vec<u8>, response: &CachedResponse) {
    let value = match serde_json::to_vec(response) {
        Ok(value) => value,
        Err(e) => return log::warn!("GetItem: could not encode response: {e}"),
    };
    match cache::set_if(key, value, GET_ITEM_TTL, SetIfCondition::Absent) {
        Ok(ConditionalSetResult::Stored(())) => {}
        Ok(ConditionalSetResult::NotStored) => {
            log::info!("GetItem: not cached; item was written during the read")
        }
        Err(e) => log::warn!("GetItem: cache set failed: {e}"),
    }
}

fn write(
    action: WriteAction,
    request: &Value,
    body: Vec<u8>,
    key_attributes: &KeyAttributes,
    forward: impl Fn(Vec<u8>) -> WebResult<CachedResponse>,
) -> WebResult<CachedResponse> {
    let result = forward(body);
    // Only a 4xx proves DynamoDB didn't apply the write. A 5xx or a failed
    // forward (e.g. a timeout after commit) may have, so invalidate anyway.
    // Cache failures are logged, not returned: an error here would make the SDK
    // retry a write that already happened.
    let rejected = matches!(&result, Ok(response) if response.is_client_error());
    if !rejected {
        for item in items_to_invalidate(action, request, key_attributes) {
            let key = item.cache_key();
            log::info!("{action:?}: invalidate {}", String::from_utf8_lossy(&key));
            tombstone::invalidate(key);
        }
    }
    result
}
