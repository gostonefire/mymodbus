//! Logging implementation for Mymodbus
//!
//! Provides a simple thread-safe logger that writes to both a file and optionally stdout.
//! It implements the `log::Log` trait and includes custom timestamp formatting.

use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{anyhow, Result};
use log::{LevelFilter, Log, Metadata, Record};

/// Contains the file and stdout handles for logging output
struct OutputTargets {
    /// Buffered file writer
    file: BufWriter<File>,
    /// Optional stdout handle
    stdout: Option<io::Stdout>,
}

/// A simple implementation of the `log::Log` trait
struct SimpleLogger {
    /// Minimum log level to record
    level: LevelFilter,
    /// Thread-safe output targets
    targets: Mutex<OutputTargets>,
}

impl SimpleLogger {
    /// Creates a new `SimpleLogger` instance
    ///
    /// # Arguments
    ///
    /// * `log_path` - path where to save logs
    /// * `level` - maximum log level to be recorded
    /// * `log_to_stdout` - whether to also output logs to stdout
    fn new(log_path: &str, level: LevelFilter, log_to_stdout: bool) -> Result<Self, io::Error> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(Path::new(log_path))?;

        Ok(Self {
            level,
            targets: Mutex::new(OutputTargets {
                file: BufWriter::new(file),
                stdout: log_to_stdout.then(io::stdout),
            }),
        })
    }

    /// Generates a UTC ISO 8601 formatted timestamp string
    ///
    /// Note: This is a manual implementation of timestamp formatting to avoid
    /// additional dependencies.
    fn timestamp() -> String {
        let secs = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_secs() as i64,
            Err(_) => 0,
        };

        let days = secs.div_euclid(86_400);
        let seconds_of_day = secs.rem_euclid(86_400);

        let hour = seconds_of_day / 3_600;
        let minute = (seconds_of_day % 3_600) / 60;
        let second = seconds_of_day % 60;

        let z = days + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let mut year = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = mp + if mp < 10 { 3 } else { -9 };
        year += if month <= 2 { 1 } else { 0 };

        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}+00:00",
            year, month, day, hour, minute, second
        )
    }

    /// Formats a log record into a single string with timestamp and level
    ///
    /// # Arguments
    ///
    /// * `record` - the log record to format
    fn format_record(&self, record: &Record<'_>) -> String {
        let timestamp = Self::timestamp();

        format!(
            "[{} {} {}] - {}\n",
            timestamp,
            record.level(),
            record.target(),
            record.args()
        )
    }
}

impl Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let line = self.format_record(record);

        let Ok(mut targets) = self.targets.lock() else {
            return;
        };

        let _ = targets.file.write_all(line.as_bytes());
        let _ = targets.file.flush();

        if let Some(stdout) = targets.stdout.as_mut() {
            let _ = stdout.write_all(line.as_bytes());
            let _ = stdout.flush();
        }
    }

    fn flush(&self) {
        let Ok(mut targets) = self.targets.lock() else {
            return;
        };

        let _ = targets.file.flush();

        if let Some(stdout) = targets.stdout.as_mut() {
            let _ = stdout.flush();
        }
    }
}

/// Initializes the global logger
///
/// This function sets up a `SimpleLogger`, leaks it to obtain a `'static` reference,
/// and registers it with the `log` crate.
///
/// # Arguments
///
/// * `log_path` - path where to save logs
/// * `level` - maximum log level to be recorded
/// * `log_to_stdout` - whether to also output logs to stdout
pub fn setup_logger(
    log_path: &str,
    level: LevelFilter,
    log_to_stdout: bool,
) -> Result<()> {
    let logger = SimpleLogger::new(log_path, level, log_to_stdout)?;
    let logger: &'static SimpleLogger = Box::leak(Box::new(logger));
    log::set_logger(logger).map_err(|_| anyhow!("failed to set logger"))?;
    log::set_max_level(level);
    Ok(())
}
