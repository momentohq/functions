use std::fmt::Write as FmtWrite;

use log::{Log, set_logger_racy, set_max_level};
use time::format_description::well_known::Rfc3339;

use crate::wit::momento::log::logging;

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

impl From<LogDestination> for logging::Destination {
    fn from(value: LogDestination) -> Self {
        match value {
            LogDestination::Topic { topic } => {
                logging::Destination::Topic(logging::TopicDestination { topic_name: topic })
            }
            LogDestination::CloudWatch {
                iam_role_arn,
                log_group_name,
            } => logging::Destination::Cloudwatch(logging::CloudwatchDestination {
                iam_role_arn,
                log_group_name,
            }),
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

impl From<LogConfiguration> for logging::ConfigureLoggingInput {
    fn from(value: LogConfiguration) -> Self {
        Self {
            log_level: level_filter_to_wit(value.log_level),
            system_logs_level: level_filter_to_wit(value.system_log_level),
            destination: value.destination.into(),
        }
    }
}

fn level_filter_to_wit(level: log::LevelFilter) -> logging::LogLevel {
    match level {
        log::LevelFilter::Off => logging::LogLevel::Off,
        log::LevelFilter::Error => logging::LogLevel::Error,
        log::LevelFilter::Warn => logging::LogLevel::Warn,
        log::LevelFilter::Info => logging::LogLevel::Info,
        log::LevelFilter::Debug => logging::LogLevel::Debug,
        // Momento does not publish Trace logs
        log::LevelFilter::Trace => logging::LogLevel::Debug,
    }
}

fn level_to_wit(level: log::Level) -> logging::LogLevel {
    match level {
        log::Level::Error => logging::LogLevel::Error,
        log::Level::Warn => logging::LogLevel::Warn,
        log::Level::Info => logging::LogLevel::Info,
        log::Level::Debug => logging::LogLevel::Debug,
        // Momento does not publish Trace logs
        log::Level::Trace => logging::LogLevel::Debug,
    }
}

/// Captures any errors Momento has detected during log configuration
#[derive(Debug, thiserror::Error)]
pub enum LogConfigurationError {
    /// Invalid auth was provided
    #[error("Invalid auth provided for configuration! {message}")]
    Auth {
        /// The error message bubbled back up
        message: String,
    },
    /// Something went wrong
    #[error("Something went wrong while trying to configure logs! {message}")]
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

/// Create a single `LogConfiguration` given a `LogDestination`.
pub fn log_configuration(destination: LogDestination) -> LogConfiguration {
    LogConfiguration::new(destination)
}

/// Entrypoint for configuring logs to be delivered to a destination(s)
pub fn configure_logs(
    configurations: impl IntoIterator<Item = LogConfiguration>,
) -> Result<(), LogConfigurationError> {
    HostLog::init(configurations)
}

pub(crate) struct HostLog {}

impl HostLog {
    pub(crate) fn init(
        configurations: impl IntoIterator<Item = LogConfiguration>,
    ) -> Result<(), LogConfigurationError> {
        static mut LOGGER: Option<HostLog> = None;
        // We're setting this to DEBUG so all logs are captured and sent to the host serving
        // the function. The host will determine whether to log the message.
        set_max_level(log::LevelFilter::Debug);
        let inputs: Vec<logging::ConfigureLoggingInput> =
            configurations.into_iter().map(Into::into).collect();
        logging::configure_logging(&inputs)?;
        #[allow(static_mut_refs)]
        #[allow(clippy::expect_used)]
        // SAFETY: concurrency requirement is satisfied by the single threaded nature
        // of the Function environment.
        unsafe {
            LOGGER.replace(HostLog {});
            set_logger_racy(LOGGER.as_mut().expect("logger just set")).map_err(|e| {
                LogConfigurationError::Unknown {
                    message: format!("Failed to configure logger! {e:?}"),
                }
            })
        }
    }
}

impl Log for HostLog {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        // Host logging will filter out logs based on level
        true
    }

    fn log(&self, record: &log::Record) {
        let mut buffer = String::with_capacity(128);
        let utc_now = time::OffsetDateTime::now_utc();
        let timestamp = utc_now.format(&Rfc3339).unwrap_or("<unknown>".to_string());
        let record_level = record.level();
        let level = record_level.as_str();
        let module = record.module_path().unwrap_or("<unknown>");
        let file = record.file().unwrap_or("<unknown>");
        let line = record.line().unwrap_or(0);
        let log_message = record.args();

        let _ = write!(
            &mut buffer,
            "{level} {timestamp} {module} {file}:{line} {log_message}"
        );

        logging::log(&buffer, level_to_wit(record_level));
    }

    fn flush(&self) {}
}
