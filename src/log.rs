use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, trace, warn, Level};
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

pub struct Logger {
    level: LogLevel,
    target: String,
}

impl Logger {
    pub fn new(target: &str) -> Self {
        Self {
            level: LogLevel::Info,
            target: target.to_string(),
        }
    }

    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }

    pub fn trace(&self, message: &str) {
        if self.level <= LogLevel::Trace {
            let span = tracing::span!(tracing::Level::TRACE, "log", target = %self.target);
            let _enter = span.enter();
            tracing::trace!("{}", message);
        }
    }

    pub fn debug(&self, message: &str) {
        if self.level <= LogLevel::Debug {
            let span = tracing::span!(tracing::Level::DEBUG, "log", target = %self.target);
            let _enter = span.enter();
            tracing::debug!("{}", message);
        }
    }

    pub fn info(&self, message: &str) {
        if self.level <= LogLevel::Info {
            let span = tracing::span!(tracing::Level::INFO, "log", target = %self.target);
            let _enter = span.enter();
            tracing::info!("{}", message);
        }
    }

    pub fn warn(&self, message: &str) {
        if self.level <= LogLevel::Warn {
            let span = tracing::span!(tracing::Level::WARN, "log", target = %self.target);
            let _enter = span.enter();
            tracing::warn!("{}", message);
        }
    }

    pub fn error(&self, message: &str) {
        if self.level <= LogLevel::Error {
            let span = tracing::span!(tracing::Level::ERROR, "log", target = %self.target);
            let _enter = span.enter();
            tracing::error!("{}", message);
        }
    }

    pub fn log(&self, level: LogLevel, message: &str) {
        match level {
            LogLevel::Trace => self.trace(message),
            LogLevel::Debug => self.debug(message),
            LogLevel::Info => self.info(message),
            LogLevel::Warn => self.warn(message),
            LogLevel::Error => self.error(message),
        }
    }

    pub fn set_level(&mut self, level: LogLevel) {
        self.level = level;
    }

    pub fn level(&self) -> LogLevel {
        self.level
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new("rust-eco")
    }
}

pub fn init_logger() -> crate::Result<()> {
    init_logger_with_level(LogLevel::Info)
}

pub fn init_logger_with_level(level: LogLevel) -> crate::Result<()> {
    let env_filter = EnvFilter::from_default_env()
        .add_directive(format!("rust_eco={}", level.to_string().to_lowercase()).parse()?);

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_span_events(FmtSpan::CLOSE)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    Ok(())
}

pub fn init_json_logger() -> crate::Result<()> {
    let env_filter = EnvFilter::from_default_env();

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    Ok(())
}

// File logger for writing logs to files
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

pub struct FileLogger {
    file_path: String,
    level: LogLevel,
    buffer: Arc<Mutex<Vec<String>>>,
}

impl FileLogger {
    pub fn new(file_path: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
            level: LogLevel::Info,
            buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }

    pub async fn log(&self, level: LogLevel, message: &str) -> crate::Result<()> {
        if level >= self.level {
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC");
            let log_entry = format!("[{}] {} - {}\n", timestamp, level, message);
            
            let mut buffer = self.buffer.lock().await;
            buffer.push(log_entry);
            
            if buffer.len() >= 100 {
                self.flush_internal(&mut buffer).await?;
            }
        }
        Ok(())
    }

    pub async fn trace(&self, message: &str) -> crate::Result<()> {
        self.log(LogLevel::Trace, message).await
    }

    pub async fn debug(&self, message: &str) -> crate::Result<()> {
        self.log(LogLevel::Debug, message).await
    }

    pub async fn info(&self, message: &str) -> crate::Result<()> {
        self.log(LogLevel::Info, message).await
    }

    pub async fn warn(&self, message: &str) -> crate::Result<()> {
        self.log(LogLevel::Warn, message).await
    }

    pub async fn error(&self, message: &str) -> crate::Result<()> {
        self.log(LogLevel::Error, message).await
    }

    pub async fn flush(&self) -> crate::Result<()> {
        let mut buffer = self.buffer.lock().await;
        self.flush_internal(&mut buffer).await
    }

    async fn flush_internal(&self, buffer: &mut Vec<String>) -> crate::Result<()> {
        if !buffer.is_empty() {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.file_path)
                .await?;

            for entry in buffer.drain(..) {
                file.write_all(entry.as_bytes()).await?;
            }
            
            file.flush().await?;
        }
        Ok(())
    }
}

// Structured logging with key-value pairs
use std::collections::HashMap;

pub struct StructuredLogger {
    base_fields: HashMap<String, String>,
    level: LogLevel,
}

impl StructuredLogger {
    pub fn new() -> Self {
        Self {
            base_fields: HashMap::new(),
            level: LogLevel::Info,
        }
    }

    pub fn with_field(mut self, key: &str, value: &str) -> Self {
        self.base_fields.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }

    pub fn log_with_fields(&self, level: LogLevel, message: &str, fields: &HashMap<String, String>) {
        if level >= self.level {
            let mut all_fields = self.base_fields.clone();
            all_fields.extend(fields.iter().map(|(k, v)| (k.clone(), v.clone())));
            
            let fields_str = all_fields
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(" ");

            let log_message = if fields_str.is_empty() {
                message.to_string()
            } else {
                format!("{} [{}]", message, fields_str)
            };

            match level {
                LogLevel::Trace => trace!("{}", log_message),
                LogLevel::Debug => debug!("{}", log_message),
                LogLevel::Info => info!("{}", log_message),
                LogLevel::Warn => warn!("{}", log_message),
                LogLevel::Error => error!("{}", log_message),
            }
        }
    }

    pub fn info_with_fields(&self, message: &str, fields: &HashMap<String, String>) {
        self.log_with_fields(LogLevel::Info, message, fields);
    }

    pub fn error_with_fields(&self, message: &str, fields: &HashMap<String, String>) {
        self.log_with_fields(LogLevel::Error, message, fields);
    }
}

impl Default for StructuredLogger {
    fn default() -> Self {
        Self::new()
    }
}

// Utility macros for easier logging
#[macro_export]
macro_rules! log_trace {
    ($logger:expr, $($arg:tt)*) => {
        $logger.trace(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($logger:expr, $($arg:tt)*) => {
        $logger.debug(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_info {
    ($logger:expr, $($arg:tt)*) => {
        $logger.info(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($logger:expr, $($arg:tt)*) => {
        $logger.warn(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($logger:expr, $($arg:tt)*) => {
        $logger.error(&format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }

    #[test]
    fn test_logger_creation() {
        let logger = Logger::new("test")
            .with_level(LogLevel::Debug);
        
        assert_eq!(logger.level(), LogLevel::Debug);
        assert_eq!(logger.target, "test");
    }

    #[tokio::test]
    async fn test_file_logger() {
        let temp_dir = tempdir().unwrap();
        let log_file = temp_dir.path().join("test.log");
        
        let logger = FileLogger::new(log_file.to_str().unwrap())
            .with_level(LogLevel::Debug);
        
        logger.info("Test info message").await.unwrap();
        logger.error("Test error message").await.unwrap();
        logger.flush().await.unwrap();
        
        let content = fs::read_to_string(&log_file).await.unwrap();
        assert!(content.contains("Test info message"));
        assert!(content.contains("Test error message"));
        assert!(content.contains("INFO"));
        assert!(content.contains("ERROR"));
    }

    #[test]
    fn test_structured_logger() {
        let logger = StructuredLogger::new()
            .with_field("service", "test")
            .with_level(LogLevel::Debug);
        
        let mut fields = HashMap::new();
        fields.insert("user_id".to_string(), "12345".to_string());
        fields.insert("action".to_string(), "login".to_string());
        
        // This test just ensures the logger doesn't panic
        logger.info_with_fields("User logged in", &fields);
    }

    #[test]
    fn test_logging_macros() {
        let logger = Logger::new("test");
        
        // Test that macros compile and don't panic
        log_info!(logger, "Test message: {}", 42);
        log_error!(logger, "Error: {}", "something went wrong");
    }

    #[tokio::test]
    async fn test_file_logger_buffering() {
        let temp_dir = tempdir().unwrap();
        let log_file = temp_dir.path().join("buffer_test.log");
        
        let logger = FileLogger::new(log_file.to_str().unwrap());
        
        // Log a few messages (should be buffered)
        for i in 0..5 {
            logger.info(&format!("Message {}", i)).await.unwrap();
        }
        
        // File should be empty or small since messages are buffered
        let initial_size = fs::metadata(&log_file).await
            .map(|m| m.len())
            .unwrap_or(0);
        
        // Flush manually
        logger.flush().await.unwrap();
        
        // Now file should contain all messages
        let final_size = fs::metadata(&log_file).await.unwrap().len();
        assert!(final_size > initial_size);
        
        let content = fs::read_to_string(&log_file).await.unwrap();
        for i in 0..5 {
            assert!(content.contains(&format!("Message {}", i)));
        }
    }

    #[tokio::test]
    async fn test_file_logger_level_filtering() {
        let temp_dir = tempdir().unwrap();
        let log_file = temp_dir.path().join("level_test.log");
        
        let logger = FileLogger::new(log_file.to_str().unwrap())
            .with_level(LogLevel::Warn);
        
        logger.debug("This should not appear").await.unwrap();
        logger.info("This should not appear either").await.unwrap();
        logger.warn("This should appear").await.unwrap();
        logger.error("This should also appear").await.unwrap();
        logger.flush().await.unwrap();
        
        let content = fs::read_to_string(&log_file).await.unwrap();
        assert!(!content.contains("This should not appear"));
        assert!(content.contains("This should appear"));
        assert!(content.contains("This should also appear"));
    }
}