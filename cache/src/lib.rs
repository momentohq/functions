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

pub use cache_interfaces::{get, set, set_if};
pub use errors::{CacheGetError, CacheSetError, CacheSetIfError};
pub use set_if::{ConditionalSetResult, SetIfCondition};
