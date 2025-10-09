//! Host interfaces for working with host logging, allowing you to send
//! logs to different destinations
use momento_functions_wit::host::momento::host::logging;
use thiserror::Error;

/// Where do you want your logs to go?
pub enum LogDestination {
    /// Momento topic within the same cache as your function
    Topic {
        /// Name of the topic
        topic: String,
    },
}

/// A single configuration for a destination
pub struct ConfigureLoggingInput {
    /// At what level would you like Momento's system logs to be filtered into this destination?
    pub system_log_level: log::LevelFilter,
    /// The specific destination
    pub destination: LogDestination,
}

impl ConfigureLoggingInput {
    /// Constructs a single logging input with a desired destination. System logs will be at default level (INFO).
    pub fn new(destination: LogDestination) -> Self {
        Self {
            system_log_level: log::LevelFilter::Info,
            destination,
        }
    }

    /// Constructs a single logging input with a desired destination as well as a specified system logs filter.
    pub fn new_with_system_log_level(
        system_log_level: log::LevelFilter,
        destination: LogDestination,
    ) -> Self {
        Self {
            system_log_level,
            destination,
        }
    }
}

impl From<LogDestination> for logging::Destination {
    fn from(value: LogDestination) -> Self {
        match value {
            LogDestination::Topic { topic } => {
                momento_functions_wit::host::momento::host::logging::Destination::Topic(
                    logging::TopicDestination { topic_name: topic },
                )
            }
        }
    }
}

impl From<ConfigureLoggingInput> for logging::ConfigureLoggingInput {
    fn from(value: ConfigureLoggingInput) -> Self {
        Self {
            system_logs_level: match value.system_log_level {
                log::LevelFilter::Off => logging::LogLevel::Off,
                log::LevelFilter::Error => logging::LogLevel::Error,
                log::LevelFilter::Warn => logging::LogLevel::Warn,
                log::LevelFilter::Info => logging::LogLevel::Info,
                log::LevelFilter::Debug => logging::LogLevel::Debug,
                // Momento does not publish Trace logs
                log::LevelFilter::Trace => logging::LogLevel::Debug,
            },
            destination: value.destination.into(),
        }
    }
}

/// Captures any errors Momento has detected during log configuration
#[derive(Debug, Error)]
pub enum LogConfigurationError {
    #[error("Invalid auth provided for configuration! {message}")]
    /// Invalid auth was provided
    Auth {
        /// The error message bubbled back up
        message: String,
    },
    #[error("Something went wrong while trying to configure logs! {message}")]
    /// Something went wrong
    Unknown {
        /// The error message bubbled back up
        message: String,
    },
}

/// Configures logging via Momento host functions
pub fn configure_logging(inputs: Vec<ConfigureLoggingInput>) -> Result<(), LogConfigurationError> {
    let converted: Vec<logging::ConfigureLoggingInput> =
        inputs.into_iter().map(Into::into).collect();
    logging::configure_logging(&converted).map_err(|e| LogConfigurationError::Auth {
        message: e.to_string(),
    })
}

/// Logs a given string
pub fn log(input: &str) {
    logging::log(input)
}
