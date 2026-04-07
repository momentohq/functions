//! Host interfaces for working with Momento Cache.
//!
//! This crate provides interfaces like get() and set(), integrated with
//! the momento-functions-bytes crate for efficient buffer management.

mod cache_interfaces;
mod errors;
mod set_if;

/// Internal module for WIT bindings.
#[doc(hidden)]
pub mod wit;

pub use cache_interfaces::{delete, get, get_with_hash, set, set_if, set_if_hash};
pub use errors::{
    CacheDeleteError, CacheGetError, CacheGetWithHashError, CacheSetError, CacheSetIfError,
    CacheSetIfHashError,
};
pub use set_if::{
    ConditionalSetResult, GetWithHashValue, SetIfCondition, SetIfHashCondition, SetIfHashResult,
};
