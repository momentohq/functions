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
    /// AWS CloudWatch Log Group for your function's logs
    CloudWatch {
        /// ARN of the IAM role for Momento to assume
        iam_role_arn: String,
        /// ARN of the CloudWatch Log Group for Momento to publish your
        /// function logs to
        log_group_name: String,
    },
}

impl LogDestination {
    /// Creates a Topic destination
    pub fn topic(name: impl Into<String>) -> Self {
        Self::Topic { topic: name.into() }
    }
    /// Creates a CloudWatch destination.
    /// Reach out to us at `support@momentohq.com` for details on how to properly
    /// set up your log configuration.
    pub fn cloudwatch(iam_role_arn: impl Into<String>, log_group_name: impl Into<String>) -> Self {
        Self::CloudWatch {
            iam_role_arn: iam_role_arn.into(),
            log_group_name: log_group_name.into(),
        }
    }
}

/// A single configuration for a destination
pub struct LogConfiguration {
    /// At what level would you like your function's logs to be filtered into this destination?
    log_level: log::LevelFilter,
    /// At what level would you like Momento's system logs to be filtered into this destination?
    system_log_level: log::LevelFilter,
    /// The specific destination
    destination: LogDestination,
}

impl LogConfiguration {
    /// Constructs a single logging input with a desired destination. System logs will be at default level (INFO).
    pub fn new(destination: LogDestination) -> Self {
        Self {
            log_level: log::LevelFilter::Info,
            system_log_level: log::LevelFilter::Info,
            destination,
        }
    }

    /// Constructs a single logging input with a desired destination as well as a specified logs filter.
    pub fn with_log_level(mut self, log_level: log::LevelFilter) -> Self {
        self.log_level = log_level;
        self
    }

    /// Constructs a single logging input with a desired destination as well as a specified system logs filter.
    pub fn with_system_log_level(mut self, system_log_level: log::LevelFilter) -> Self {
        self.system_log_level = system_log_level;
        self
    }
}

impl From<LogDestination> for LogConfiguration {
    fn from(value: LogDestination) -> Self {
        match value {
            LogDestination::Topic { topic } => Self::new(LogDestination::topic(topic)),
            LogDestination::CloudWatch {
                iam_role_arn,
                log_group_name,
            } => Self::new(LogDestination::cloudwatch(iam_role_arn, log_group_name)),
        }
    }
}

/// Create a single `LogConfiguration` given a `LogDestination`.
pub fn log_configuration(destination: LogDestination) -> LogConfiguration {
    LogConfiguration::new(destination)
}

impl From<LogDestination> for logging::Destination {
    fn from(value: LogDestination) -> Self {
        match value {
            LogDestination::Topic { topic } => {
                momento_functions_wit::host::momento::host::logging::Destination::Topic(
                    logging::TopicDestination { topic_name: topic },
                )
            }
            LogDestination::CloudWatch {
                iam_role_arn,
                log_group_name,
            } => momento_functions_wit::host::momento::host::logging::Destination::Cloudwatch(
                logging::CloudwatchDestination {
                    iam_role_arn,
                    log_group_name,
                },
            ),
        }
    }
}

impl From<LogConfiguration> for logging::ConfigureLoggingInput {
    fn from(value: LogConfiguration) -> Self {
        Self {
            log_level: match value.log_level {
                log::LevelFilter::Off => logging::LogLevel::Off,
                log::LevelFilter::Error => logging::LogLevel::Error,
                log::LevelFilter::Warn => logging::LogLevel::Warn,
                log::LevelFilter::Info => logging::LogLevel::Info,
                log::LevelFilter::Debug => logging::LogLevel::Debug,
                // Momento does not publish Trace logs
                log::LevelFilter::Trace => logging::LogLevel::Debug,
            },
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

impl From<logging::LogConfigurationError> for LogConfigurationError {
    fn from(value: logging::LogConfigurationError) -> Self {
        match value {
            logging::LogConfigurationError::Auth(e) => Self::Auth { message: e },
        }
    }
}

/// Configures logging via Momento host functions
pub fn configure_host_logging(
    configurations: impl IntoIterator<Item = LogConfiguration>,
) -> Result<(), LogConfigurationError> {
    let configurations = configurations
        .into_iter()
        .map(|configuration| configuration.into())
        .collect::<Vec<logging::ConfigureLoggingInput>>();
    Ok(logging::configure_logging(&configurations)?)
}

/// Logs a given string
pub fn log(input: &str, level: log::Level) {
    logging::log(
        input,
        match level {
            log::Level::Error => logging::LogLevel::Error,
            log::Level::Warn => logging::LogLevel::Warn,
            log::Level::Info => logging::LogLevel::Info,
            log::Level::Debug => logging::LogLevel::Debug,
            // Momento does not publish Trace logs
            log::Level::Trace => logging::LogLevel::Debug,
        },
    )
}
