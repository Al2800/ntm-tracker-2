//! Structured logging with configurable output format and file rotation.

use crate::config::LoggingConfig;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

/// Initialize the logging subsystem based on configuration.
///
/// Returns a guard that must be held for the duration of the program
/// to ensure all logs are flushed.
pub fn init(config: &LoggingConfig) -> Option<WorkerGuard> {
    let filter = build_filter(&config.level);

    match (config.file.as_ref(), config.format.as_str()) {
        // JSON to file
        (Some(path), "json") => {
            let (writer, guard) = create_file_writer(path, config);
            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_writer(writer)
                        .with_span_events(FmtSpan::CLOSE)
                        .with_filter(filter),
                )
                .init();
            Some(guard)
        }
        // Text to file
        (Some(path), _) => {
            let (writer, guard) = create_file_writer(path, config);
            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_writer(writer)
                        .with_span_events(FmtSpan::CLOSE)
                        .with_filter(filter),
                )
                .init();
            Some(guard)
        }
        // JSON to stdout
        (None, "json") => {
            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_writer(io::stdout)
                        .with_span_events(FmtSpan::CLOSE)
                        .with_filter(filter),
                )
                .init();
            None
        }
        // Text to stdout (default)
        (None, _) => {
            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_writer(io::stdout)
                        .with_span_events(FmtSpan::CLOSE)
                        .with_filter(filter),
                )
                .init();
            None
        }
    }
}

/// Build an EnvFilter from the configured log level.
fn build_filter(level: &str) -> EnvFilter {
    // Allow RUST_LOG to override config
    if std::env::var("RUST_LOG").is_ok() {
        return EnvFilter::from_default_env();
    }

    // Parse level string
    let level = match level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" | "warning" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    // Build filter with per-crate overrides for noisy dependencies
    let filter_str = format!(
        "{},hyper=warn,tokio_tungstenite=warn,tungstenite=warn",
        level
    );

    EnvFilter::try_new(&filter_str).unwrap_or_else(|_| EnvFilter::new("info"))
}

/// Create a rolling file writer.
fn create_file_writer(
    path: &Path,
    config: &LoggingConfig,
) -> (tracing_appender::non_blocking::NonBlocking, WorkerGuard) {
    let writer = RotatingFileWriter::new(path.to_path_buf(), config).unwrap_or_else(|e| {
        eprintln!(
            "Warning: Could not create rotating log writer for {}: {e}",
            path.display()
        );
        RotatingFileWriter::stdout_fallback()
    });

    tracing_appender::non_blocking(writer)
}

struct RotatingFileWriter {
    base_path: PathBuf,
    max_bytes: u64,
    max_files: usize,
    file: Option<std::fs::File>,
    written_bytes: u64,
    stdout_fallback: bool,
}

impl RotatingFileWriter {
    fn new(base_path: PathBuf, config: &LoggingConfig) -> io::Result<Self> {
        let max_bytes = config
            .max_file_mb
            .saturating_mul(1024)
            .saturating_mul(1024);

        let dir = base_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!(
                "Warning: Could not create log directory {}: {e}",
                dir.display()
            );
        }

        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&base_path)?;
        let written_bytes = file.metadata().map(|m| m.len()).unwrap_or(0);

        Ok(Self {
            base_path,
            max_bytes,
            max_files: config.max_files.max(1),
            file: Some(file),
            written_bytes,
            stdout_fallback: false,
        })
    }

    fn stdout_fallback() -> Self {
        Self {
            base_path: PathBuf::new(),
            max_bytes: 0,
            max_files: 0,
            file: None,
            written_bytes: 0,
            stdout_fallback: true,
        }
    }

    fn rotated_path(&self, index: usize) -> PathBuf {
        let Some(file_name) = self.base_path.file_name() else {
            return PathBuf::from(format!("{}.{}", self.base_path.display(), index));
        };

        let rotated_name = format!("{}.{}", file_name.to_string_lossy(), index);
        self.base_path.with_file_name(rotated_name)
    }

    fn rotate(&mut self) -> io::Result<()> {
        if self.stdout_fallback {
            return Ok(());
        }

        if let Some(mut file) = self.file.take() {
            let _ = file.flush();
        }

        // Remove the oldest rotated file if present.
        if self.max_files > 0 {
            let oldest = self.rotated_path(self.max_files);
            if oldest.exists() {
                let _ = std::fs::remove_file(&oldest);
            }
        }

        // Shift rotated files up: N-1 -> N, ..., 1 -> 2
        if self.max_files > 1 {
            for idx in (1..self.max_files).rev() {
                let src = self.rotated_path(idx);
                if !src.exists() {
                    continue;
                }
                let dst = self.rotated_path(idx + 1);
                if dst.exists() {
                    let _ = std::fs::remove_file(&dst);
                }
                let _ = std::fs::rename(&src, &dst);
            }
        }

        // Move current log to .1
        let first = self.rotated_path(1);
        if first.exists() {
            let _ = std::fs::remove_file(&first);
        }
        if self.base_path.exists() {
            let _ = std::fs::rename(&self.base_path, &first);
        }

        // Re-open fresh log file.
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.base_path)?;

        self.written_bytes = 0;
        self.file = Some(file);
        Ok(())
    }
}

impl Write for RotatingFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.stdout_fallback {
            return io::stdout().write(buf);
        }

        if self.max_bytes > 0 && self.written_bytes.saturating_add(buf.len() as u64) > self.max_bytes
        {
            self.rotate()?;
        }

        let file = self.file.as_mut().expect("log file");
        let written = file.write(buf)?;
        self.written_bytes = self.written_bytes.saturating_add(written as u64);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.stdout_fallback {
            return io::stdout().flush();
        }

        if let Some(file) = self.file.as_mut() {
            file.flush()
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_filter_parses_levels() {
        // These should not panic
        let _ = build_filter("trace");
        let _ = build_filter("debug");
        let _ = build_filter("info");
        let _ = build_filter("warn");
        let _ = build_filter("error");
        let _ = build_filter("INVALID");
    }

    #[test]
    fn default_config_is_valid() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, "info");
        assert_eq!(config.format, "text");
        assert!(config.file.is_none());
    }

    #[test]
    fn rotating_writer_rotates_by_size() {
        let dir = std::env::temp_dir().join(format!(
            "ntm-tracker-logging-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("create temp log dir");
        let path = dir.join("daemon.log");

        let config = LoggingConfig {
            file: Some(path.clone()),
            max_file_mb: 1, // not used in test logic
            max_files: 3,
            ..Default::default()
        };

        let mut writer = RotatingFileWriter::new(path.clone(), &config).expect("writer");
        writer.max_bytes = 32; // force rotation quickly

        writer.write_all(b"01234567890123456789012345678901").unwrap();
        writer.write_all(b"X").unwrap();
        writer.flush().unwrap();

        assert!(path.exists(), "current log exists");
        assert!(writer.rotated_path(1).exists(), "rotated log exists");
    }
}
