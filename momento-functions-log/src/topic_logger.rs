use log::{LevelFilter, Log, Metadata, Record, SetLoggerError, set_logger_racy, set_max_level};
use std::{fmt::Write, sync::atomic::AtomicU32};

pub struct TopicLog {
    level: LevelFilter,
    topic: String,
    dropped: AtomicU32,
}

impl TopicLog {
    pub fn init(log_level: LevelFilter, topic: String) -> Result<(), SetLoggerError> {
        set_max_level(log_level);
        static mut LOGGER: Option<TopicLog> = None;

        // SAFETY: concurrency requirement is satisfied by the single threaded nature
        // of the Function environment.
        #[allow(static_mut_refs)]
        #[allow(clippy::expect_used)]
        unsafe {
            LOGGER.replace(TopicLog {
                level: log_level,
                topic,
                dropped: AtomicU32::new(0),
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

            let dropped = self.dropped.swap(0, std::sync::atomic::Ordering::Relaxed);
            let dropped_clause = if 0 < dropped {
                format!(" ({dropped} messages dropped)")
            } else {
                String::new()
            };

            let _ = write!(
                &mut buffer,
                "{level} {module} {file}:{line}{dropped_clause} {log_message}"
            );

            if momento_functions_host::topics::publish(&self.topic, buffer.as_str()).is_err() {
                // An optimistic hint to help raise awareness of dropped messages. Probably it is due
                // to high-frequency logging and a low topic rate limit.
                // Put back the drop count if the publish failed - this way we'll keep trying to mention
                // the dropped messages.
                self.dropped
                    .fetch_add(1 + dropped, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    fn flush(&self) {}
}
