//! Off-guest buffer management.
//!
//! This crate provides the `Data` type, which is a buffer of bytes that may be
//! stored on the host. This allows you to pass data from a request or response
//! to another request or response without copying it into your function's memory.
//! This can improve performance for large buffers, when you're passing data through.

mod data;
/// Internal module for WIT bindings.
#[doc(hidden)]
pub mod wit;

pub use data::Data;
pub mod encoding;
