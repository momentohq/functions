//! Host interface tools for Momento Functions
//!
//! This crate helps you write Momento Functions.
//!
//! Functions use `wasm32-unknown-unknown` as the target architecture.
//! They use the [WIT](https://component-model.bytecodealliance.org/design/wit.html) [Component Model](https://component-model.bytecodealliance.org/)
//! to describe the ABI.
//!
//! You are likely to be interested in the sibling crates:
//! * [`momento-functions`](https://crates.io/crates/momento-functions): Code generators for Functions.
//! * [`momento-functions-log`](https://crates.io/crates/momento-functions-log): Standard `log` adapter.

pub mod cache;
mod error;
pub mod topics;

pub use error::{Error, FunctionResult};
