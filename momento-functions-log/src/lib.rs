//! `log` adapter for Momento Functions
//!
//! This crate adapts [`log`](https://docs.rs/log) to Momento Functions. `log` is a standard logging crate,
//! used widely across the ecosystem.
//!
//! Functions use `wasm32-unknown-unknown` as the target architecture.
//! They use the [WIT](https://component-model.bytecodealliance.org/design/wit.html) [Component Model](https://component-model.bytecodealliance.org/)
//! to describe the ABI.
//!
//! You are likely to be interested in the sibling crates:
//! * [`momento-functions`](https://crates.io/crates/momento-functions): Code generators for Functions.
//! * [`momento-functions-host`](https://crates.io/crates/momento-functions-host): Interfaces and tools for calling host interfaces.

use momento_functions_host::FunctionResult;

mod topic_logger;

/// Which logging mode to use?
pub enum LogMode {
    Topic {
        /// The topic to send logs to.
        ///
        /// You can get the logs with the `momento` CLI, or on the Momento topics dashboard at gomomento.com.
        /// The CLI command would be `momento topic subscribe $topic`
        /// Log messages will stream to your terminal.
        topic: String,
    },
}

/// Initializes the logging system with the specified log level and mode.
///
/// You **must** only call this function once.
pub fn configure_logging(level: log::LevelFilter, mode: LogMode) -> FunctionResult<()> {
    match mode {
        LogMode::Topic { topic } => topic_logger::TopicLog::init(level, topic).map_err(|e| {
            momento_functions_host::Error::MessageError(format!("Failed to set logger: {e}"))
        }),
    }
}
