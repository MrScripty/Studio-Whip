//! Core types for the logging service

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Log severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogLevel {
    /// Detailed information for debugging
    Trace,
    /// General debugging information
    Debug,
    /// Informational messages
    Info,
    /// Warning messages
    Warn,
    /// Error messages
    Error,
}

impl LogLevel {
    /// Convert from tracing Level
    pub fn from_tracing(level: &tracing::Level) -> Self {
        match *level {
            tracing::Level::TRACE => LogLevel::Trace,
            tracing::Level::DEBUG => LogLevel::Debug,
            tracing::Level::INFO => LogLevel::Info,
            tracing::Level::WARN => LogLevel::Warn,
            tracing::Level::ERROR => LogLevel::Error,
        }
    }
    
    /// Get color for terminal display
    pub fn color(&self) -> &'static str {
        match self {
            LogLevel::Trace => "\x1b[90m", // Gray
            LogLevel::Debug => "\x1b[36m", // Cyan
            LogLevel::Info => "\x1b[32m",  // Green
            LogLevel::Warn => "\x1b[33m",  // Yellow
            LogLevel::Error => "\x1b[31m", // Red
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// Metadata associated with a log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMetadata {
    /// Timestamp when the log was created
    pub timestamp: SystemTime,
    /// Thread ID that generated the log
    pub thread_id: u64,
    /// Source file where the log originated
    pub file: Option<String>,
    /// Line number in the source file
    pub line: Option<u32>,
    /// Target/module that generated the log
    pub target: String,
    /// Category for filtering (e.g., "rendering", "layout", "input")
    pub category: Option<String>,
}

impl LogMetadata {
    /// Create new metadata with current timestamp
    pub fn new(target: String) -> Self {
        // Use a simple hash of thread ID since as_u64().get() is unstable
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        std::thread::current().id().hash(&mut hasher);
        let thread_id = hasher.finish();
        
        Self {
            timestamp: SystemTime::now(),
            thread_id,
            file: None,
            line: None,
            target,
            category: None,
        }
    }
    
    /// Set the source location
    pub fn with_location(mut self, file: String, line: u32) -> Self {
        self.file = Some(file);
        self.line = Some(line);
        self
    }
    
    /// Set the category
    pub fn with_category(mut self, category: String) -> Self {
        self.category = Some(category);
        self
    }
}

/// Complete log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogData {
    /// Unique ID for this log entry
    pub id: u64,
    /// Log severity level
    pub level: LogLevel,
    /// Log message content
    pub message: String,
    /// Associated metadata
    pub metadata: LogMetadata,
    /// Deduplication count (if this log was repeated)
    pub duplicate_count: u32,
}

impl LogData {
    /// Create a new log entry
    pub fn new(id: u64, level: LogLevel, message: String, metadata: LogMetadata) -> Self {
        Self {
            id,
            level,
            message,
            metadata,
            duplicate_count: 0,
        }
    }
    
    /// Check if this log is a duplicate of another
    pub fn is_duplicate_of(&self, other: &LogData) -> bool {
        self.level == other.level
            && self.message == other.message
            && self.metadata.target == other.metadata.target
            && self.metadata.file == other.metadata.file
            && self.metadata.line == other.metadata.line
    }
    
    /// Format the log for display
    pub fn format_display(&self) -> String {
        let timestamp = self.metadata.timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        
        let location = match (&self.metadata.file, self.metadata.line) {
            (Some(file), Some(line)) => format!(" {}:{}", file, line),
            _ => String::new(),
        };
        
        let duplicate = if self.duplicate_count > 0 {
            format!(" ({}x)", self.duplicate_count + 1)
        } else {
            String::new()
        };
        
        format!(
            "[{}.{:03}] {} [{}]{}{} {}",
            timestamp.as_secs(),
            timestamp.subsec_millis(),
            self.level,
            self.metadata.target,
            location,
            duplicate,
            self.message
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }
    
    #[test]
    fn test_log_duplicate_detection() {
        let meta1 = LogMetadata::new("test".to_string())
            .with_location("test.rs".to_string(), 42);
        let meta2 = LogMetadata::new("test".to_string())
            .with_location("test.rs".to_string(), 42);
        
        let log1 = LogData::new(1, LogLevel::Info, "Test message".to_string(), meta1);
        let log2 = LogData::new(2, LogLevel::Info, "Test message".to_string(), meta2);
        
        assert!(log1.is_duplicate_of(&log2));
    }
}