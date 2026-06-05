//! Host interfaces for making HTTP requests from Momento Functions.
//!
//! This crate provides a typed API for sending HTTP requests, with support
//! for custom headers, request bodies, and AWS authorization strategies.

mod invoke;
mod request;
pub mod sse;

/// Internal module for WIT bindings.
#[doc(hidden)]
pub mod wit;

pub use invoke::{HttpError, Response, invoke};
pub use momento_functions_bytes::Data;
pub use request::{Authorization, AwsSigV4Secret, IamRole, Request};
