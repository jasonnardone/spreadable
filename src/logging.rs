// Logging infrastructure using tracing
// Supports JSON and pretty formatting with file rotation

use crate::config::LoggingConfig;
use crate::error::{Result, SpreadableError};
use std::fs;
use std::path::Path;
use tracing::Level;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

/// Initialize logging based on configuration
pub fn init_logging(config: &LoggingConfig) -> Result<()> {
    // Parse log level
    let log_level = parse_log_level(&config.level)?;

    // Create log directory if it doesn't exist
    if let Some(parent) = Path::new(&config.file_path).parent() {
        fs::create_dir_all(parent).map_err(|e| {
            SpreadableError::Config(format!("Failed to create log directory: {e}"))
        })?;
    }

    // Build environment filter
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(config.level.clone()))
        .add_directive(format!("spreadable={}", log_level).parse().unwrap());

    // Create file appender with rotation
    let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(match config.file_rotation.as_str() {
            "daily" => tracing_appender::rolling::Rotation::DAILY,
            "hourly" => tracing_appender::rolling::Rotation::HOURLY,
            _ => tracing_appender::rolling::Rotation::NEVER,
        })
        .max_log_files(7) // Keep last 7 files
        .filename_prefix("spreadable")
        .filename_suffix("log")
        .build(
            Path::new(&config.file_path)
                .parent()
                .unwrap_or_else(|| Path::new(".")),
        )
        .map_err(|e| {
            SpreadableError::Config(format!("Failed to create file appender: {e}"))
        })?;

    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

    // Build layers based on format
    match config.format.as_str() {
        "json" => {
            // JSON format for production
            let file_layer = fmt::layer()
                .json()
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_target(true)
                .with_span_events(FmtSpan::CLOSE)
                .with_writer(non_blocking_file)
                .with_filter(env_filter.clone());

            let stdout_layer = fmt::layer()
                .json()
                .with_writer(std::io::stdout)
                .with_filter(env_filter);

            tracing_subscriber::registry()
                .with(file_layer)
                .with(stdout_layer)
                .init();
        }
        "pretty" => {
            // Pretty format for development
            let file_layer = fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(false)
                .with_target(true)
                .with_ansi(false) // No ANSI colors in file
                .with_writer(non_blocking_file)
                .with_filter(env_filter.clone());

            let stdout_layer = fmt::layer()
                .pretty()
                .with_thread_ids(false)
                .with_target(true)
                .with_writer(std::io::stdout)
                .with_filter(env_filter);

            tracing_subscriber::registry()
                .with(file_layer)
                .with(stdout_layer)
                .init();
        }
        _ => {
            return Err(SpreadableError::Config(format!(
                "Invalid log format: {}. Must be 'json' or 'pretty'",
                config.format
            )));
        }
    }

    tracing::info!(
        "Logging initialized: level={}, format={}, file={}",
        config.level,
        config.format,
        config.file_path
    );

    Ok(())
}

fn parse_log_level(level: &str) -> Result<Level> {
    match level.to_lowercase().as_str() {
        "error" => Ok(Level::ERROR),
        "warn" => Ok(Level::WARN),
        "info" => Ok(Level::INFO),
        "debug" => Ok(Level::DEBUG),
        "trace" => Ok(Level::TRACE),
        _ => Err(SpreadableError::Config(format!(
            "Invalid log level: {}. Must be one of: error, warn, info, debug, trace",
            level
        ))),
    }
}

/// Log a structured event with custom fields
#[macro_export]
macro_rules! log_event {
    ($level:expr, $event_type:expr, $($key:tt = $value:expr),+ $(,)?) => {
        match $level {
            tracing::Level::ERROR => tracing::error!(event_type = $event_type, $($key = $value),+),
            tracing::Level::WARN => tracing::warn!(event_type = $event_type, $($key = $value),+),
            tracing::Level::INFO => tracing::info!(event_type = $event_type, $($key = $value),+),
            tracing::Level::DEBUG => tracing::debug!(event_type = $event_type, $($key = $value),+),
            tracing::Level::TRACE => tracing::trace!(event_type = $event_type, $($key = $value),+),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_level() {
        assert!(matches!(parse_log_level("error"), Ok(Level::ERROR)));
        assert!(matches!(parse_log_level("warn"), Ok(Level::WARN)));
        assert!(matches!(parse_log_level("info"), Ok(Level::INFO)));
        assert!(matches!(parse_log_level("debug"), Ok(Level::DEBUG)));
        assert!(matches!(parse_log_level("trace"), Ok(Level::TRACE)));

        // Case insensitive
        assert!(matches!(parse_log_level("ERROR"), Ok(Level::ERROR)));
        assert!(matches!(parse_log_level("Info"), Ok(Level::INFO)));

        // Invalid level
        assert!(parse_log_level("invalid").is_err());
    }

    #[test]
    fn test_invalid_format() {
        let config = LoggingConfig {
            level: "info".to_string(),
            format: "invalid".to_string(),
            file_path: "/tmp/test.log".to_string(),
            file_rotation: "daily".to_string(),
            file_max_size_mb: 100,
        };

        let result = init_logging(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid log format"));
    }
}
