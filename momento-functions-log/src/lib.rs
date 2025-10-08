//! `log` adapter for Momento Functions
//!
//! This crate adapts [`log`](https://docs.rs/log) to Momento Functions. `log` is a standard logging crate,
//! used widely across the ecosystem.
//!
//! Functions use `wasm32-wasip2` as the target architecture.
//! They use the [WIT](https://component-model.bytecodealliance.org/design/wit.html) [Component Model](https://component-model.bytecodealliance.org/)
//! to describe the ABI.
//!
//! You are likely to be interested in the sibling crates:
//! * [`momento-functions`](https://crates.io/crates/momento-functions): Code generators for Functions.
//! * [`momento-functions-host`](https://crates.io/crates/momento-functions-host): Interfaces and tools for calling host interfaces.

use momento_functions_host::logging::{ConfigureLoggingInput, LogConfigurationError};
use thiserror::Error;

mod host_logging;

#[derive(Debug, Error)]
pub enum LogConfigError {
    #[error("Failed to initialize logger: {cause}")]
    Init { cause: LogConfigurationError },
}

/// Initializes the logging system with the specified log level and destinations.
///
/// You **must** only call this function once.
pub fn configure_logging(
    level: log::LevelFilter,
    destinations: Vec<ConfigureLoggingInput>,
) -> Result<(), LogConfigError> {
    host_logging::HostLog::init(level, destinations).map_err(|e| LogConfigError::Init { cause: e })
}
