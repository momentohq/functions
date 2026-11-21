//! Shared caching logic for the DynamoDB caching Function prototypes
//! (`dynamodb-cache-proxy` and `dynamodb-cache-federated`). The two Functions
//! differ only in how they authenticate the request they forward to DynamoDB;
//! everything else lives here.

mod bad_request;
mod cache_key;
mod cached_response;
mod dispatch;
mod dynamodb_endpoint;
mod forward;
mod get_item;
mod headers;
mod invalidation;
mod item_ref;
mod key_attributes;
mod tombstone;
mod write_action;

// What the Functions use.
pub use bad_request::BadRequest;
pub use cached_response::CachedResponse;
pub use dispatch::dispatch;
pub use dynamodb_endpoint::is_dynamodb_url;
pub use forward::forward;
pub use headers::{require_header, without_headers};
pub use key_attributes::KeyAttributes;

// Shared between this crate's modules only.
use get_item::cacheable_get_item;
use invalidation::items_to_invalidate;
use item_ref::ItemRef;
use write_action::WriteAction;
