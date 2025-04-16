//! WIT code generation root for Momento Functions
//!
//! This crate is an internal library for generating Momento Functions webassembly bindings.
//! It is not intended for use by end users, and may be materially changed at any time.
//!
//! Functions use `wasm32-unknown-unknown` as the target architecture.
//! They use the [WIT](https://component-model.bytecodealliance.org/design/wit.html) [Component Model](https://component-model.bytecodealliance.org/)
//! to describe the ABI.
//!
//! You are likely to be interested in the sibling crates:
//! * [`momento-functions`](https://crates.io/crates/momento-functions): Code generators for Functions.
//! * [`momento-functions-host`](https://crates.io/crates/momento-functions-host): Interfaces and tools for calling host interfaces.
//! * [`momento-functions-log`](https://crates.io/crates/momento-functions-log): Standard `log` adapter.

pub mod function_spawn;
pub mod function_web;
pub mod host;
