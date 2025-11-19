use std::fmt::Write;

use log::{Log, set_logger_racy, set_max_level};
use momento_functions_host::logging::{LogConfiguration, LogConfigurationError};
use time::format_description::well_known::Rfc3339;

pub struct HostLog {}

impl HostLog {
    pub fn init(
        configurations: impl IntoIterator<Item = LogConfiguration>,
    ) -> Result<(), LogConfigurationError> {
        static mut LOGGER: Option<HostLog> = None;
        // We're setting this to DEBUG so all logs are captured and sent to the host serving
        // the function. The host will determine whether to log the mesage.
        set_max_level(log::LevelFilter::Debug);
        momento_functions_host::logging::configure_host_logging(configurations)?;
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

        momento_functions_host::logging::log(buffer.as_str(), record_level);
    }

    fn flush(&self) {}
}
