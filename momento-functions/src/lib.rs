//! Code generation helpers for Momento Functions
//!
//! This crate generates Momento Functions webassembly bindings.
//!
//! Functions use `wasm32-unknown-unknown` as the target architecture.
//! They use the [WIT](https://component-model.bytecodealliance.org/design/wit.html) [Component Model](https://component-model.bytecodealliance.org/)
//! to describe the ABI.
//!
//! You are likely to be interested in the sibling crates:
//! * [`momento-functions-host`](https://crates.io/crates/momento-functions-host): Interfaces and tools for calling host interfaces.
//! * [`momento-functions-log`](https://crates.io/crates/momento-functions-log): Standard `log` adapter.
mod macros;
