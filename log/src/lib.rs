//! `log` adapter for Momento Functions
//!
//! This crate adapts [`log`](https://docs.rs/log) to Momento Functions. `log` is a standard
//! logging crate used widely across the ecosystem.
//!
//! Functions use `wasm32-wasip2` as the target architecture.
//! They use the [WIT](https://component-model.bytecodealliance.org/design/wit.html) [Component Model](https://component-model.bytecodealliance.org/)
//! to describe the ABI.
//!
//! You are likely to be interested in the sibling crates:
//! * [`momento-functions`](https://crates.io/crates/momento-functions): Code generators for Functions.
//! * [`momento-functions-host`](https://crates.io/crates/momento-functions-host): Interfaces and tools for calling host interfaces.

mod host_logging;

/// Internal module for WIT bindings.
#[doc(hidden)]
pub mod wit;

pub use host_logging::{
    LogConfiguration, LogConfigurationError, LogDestination, configure_logs, log_configuration,
};
