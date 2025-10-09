use std::fmt::Write;
use std::sync::atomic::AtomicU32;

use log::{LevelFilter, Log, set_logger_racy, set_max_level};
use momento_functions_host::logging::{ConfigureLoggingInput, LogConfigurationError};
use time::format_description::well_known::Rfc3339;

pub struct HostLog {
    level: LevelFilter,
    dropped: AtomicU32,
}

impl HostLog {
    pub fn init(
        log_level: LevelFilter,
        destinations: Vec<ConfigureLoggingInput>,
    ) -> Result<(), LogConfigurationError> {
        set_max_level(log_level);

        static mut LOGGER: Option<HostLog> = None;
        momento_functions_host::logging::configure_logging(destinations)?;
        #[allow(static_mut_refs)]
        #[allow(clippy::expect_used)]
        unsafe {
            LOGGER.replace(HostLog {
                level: log_level,
                dropped: AtomicU32::new(0),
            });
            set_logger_racy(LOGGER.as_mut().expect("logger just set")).map_err(|e| {
                LogConfigurationError::Unknown {
                    message: format!("Failed to configure logger! {e:?}"),
                }
            })
        }
    }
}

impl Log for HostLog {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let mut buffer = String::with_capacity(128);
            let utc_now = time::OffsetDateTime::now_utc();
            let timestamp = utc_now.format(&Rfc3339).unwrap_or("<unknown>".to_string());
            let level = record.level().as_str();
            let module = record.module_path().unwrap_or("<unknown>");
            let file = record.file().unwrap_or("<unknown>");
            let line = record.line().unwrap_or(0);
            let log_message = record.args();

            let dropped = self.dropped.swap(0, std::sync::atomic::Ordering::Relaxed);
            let dropped_clause = if 0 < dropped {
                format!(" ({dropped} messages dropped)")
            } else {
                String::new()
            };

            let _ = write!(
                &mut buffer,
                "{level} {timestamp} {module} {file}:{line}{dropped_clause} {log_message}"
            );

            momento_functions_host::logging::log(buffer.as_str());
        }
    }

    fn flush(&self) {}
}
