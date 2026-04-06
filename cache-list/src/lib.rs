//! Host interfaces for working with Momento Cache lists.
//!
//! This crate provides list operations like push, pop, fetch, and more,
//! integrated with the momento-functions-bytes crate for efficient buffer management.

mod collection_ttl;
mod errors;
mod list_interfaces;
mod types;

/// Internal module for WIT bindings.
#[doc(hidden)]
pub mod wit;

pub use collection_ttl::CollectionTtl;
pub use errors::{
    CacheListConcatenateError, CacheListEraseError, CacheListFetchError, CacheListLengthError,
    CacheListPopError, CacheListPushBackError, CacheListPushFrontError, CacheListRemoveError,
    CacheListRetainError,
};
pub use list_interfaces::{
    list_concatenate_back, list_concatenate_front, list_erase, list_fetch, list_length,
    list_pop_back, list_pop_front, list_push_back, list_push_front, list_remove, list_retain,
};
pub use types::{
    EndIndex, EraseRange, EraseResult, LengthResult, ListRange, PopResult, RemoveRange,
    RemoveResult, RetainResult, StartIndex,
};
