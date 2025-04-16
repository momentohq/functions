use log::{LevelFilter, Log, Metadata, Record, SetLoggerError, set_logger_racy, set_max_level};
use std::fmt::Write;

pub struct TopicLog {
    level: LevelFilter,
    topic: String,
}

impl TopicLog {
    pub fn init(log_level: LevelFilter, topic: String) -> Result<(), SetLoggerError> {
        set_max_level(log_level);
        static mut LOGGER: Option<TopicLog> = None;

        // SAFETY: concurrency requirement is satisfied by the single threaded nature
        // of the Function environment.
        #[allow(static_mut_refs)]
        unsafe {
            LOGGER.replace(TopicLog {
                level: log_level,
                topic,
            });
            set_logger_racy(LOGGER.as_mut().expect("it was just set"))
        }
    }
}

impl Log for TopicLog {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            let mut buffer = String::with_capacity(128);
            let level = record.level().as_str();
            let module = record.module_path().unwrap_or("<unknown>");
            let file = record.file().unwrap_or("<unknown>");
            let line = record.line().unwrap_or(0);
            let log_message = record.args();
            let _ = write!(&mut buffer, "{level} {module} {file}:{line} {log_message}");

            let _ = momento_functions_host::topics::publish(&self.topic, &buffer);
        }
    }

    fn flush(&self) {}
}
