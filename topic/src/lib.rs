//! Host interfaces for publishing messages to Momento topics.
//!
//! This crate provides functions for publishing string and binary messages
//! to Momento topics.

mod publish;

/// Internal module for WIT bindings.
#[doc(hidden)]
pub mod wit;

pub use publish::{TopicError, publish, publish_bytes};
