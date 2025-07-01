//! Central log storage with thread-safe access and deduplication

use crate::logging::types::{LogData, LogLevel, LogMetadata};
use crate::logging::filter::{LogFilter, FilterConfig};
use crossbeam_channel::{bounded, Sender};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime};

/// Statistics about the log store
#[derive(Debug, Clone)]
pub struct LogStoreStats {
    /// Total number of logs ever added
    pub total_logs: u64,
    /// Number of logs currently stored  
    pub current_logs: usize,
    /// Number of duplicate logs detected
    pub duplicates_detected: u64,
    /// Storage capacity
    pub capacity: usize,
    /// Number of logs dropped due to capacity
    pub logs_dropped: u64,
}

/// Internal message types for the log store worker
#[derive(Debug)]
enum LogStoreMessage {
    AddLog(LogData),
    GetLogs {
        filter: LogFilter,
        start: usize,
        count: usize,
        response: crossbeam_channel::Sender<Vec<LogData>>,
    },
    GetStats {
        response: crossbeam_channel::Sender<LogStoreStats>,
    },
    UpdateFilter(FilterConfig),
    Clear,
    Shutdown,
}

/// Thread-safe central log storage
pub struct CentralLogStore {
    sender: Sender<LogStoreMessage>,
    _worker_handle: thread::JoinHandle<()>,
}

/// Internal worker state
struct LogStoreWorker {
    logs: VecDeque<LogData>,
    capacity: usize,
    next_id: AtomicU64,
    total_logs: u64,
    duplicates_detected: u64,
    logs_dropped: u64,
    filter: LogFilter,
    last_cleanup: SystemTime,
}

impl LogStoreWorker {
    fn new(capacity: usize) -> Self {
        Self {
            logs: VecDeque::with_capacity(capacity),
            capacity,
            next_id: AtomicU64::new(1),
            total_logs: 0,
            duplicates_detected: 0,
            logs_dropped: 0,
            filter: LogFilter::default(),
            last_cleanup: SystemTime::now(),
        }
    }
    
    fn add_log(&mut self, mut log: LogData) {
        self.total_logs += 1;
        
        // Set unique ID
        log.id = self.next_id.fetch_add(1, Ordering::Relaxed);
        
        // Check for duplicates (only check recent logs for performance)
        let check_count = std::cmp::min(self.logs.len(), 100);
        if let Some(duplicate_pos) = self.logs.iter().rev().take(check_count)
            .position(|existing| log.is_duplicate_of(existing)) {
            
            // Update existing log's duplicate count
            let pos = self.logs.len() - 1 - duplicate_pos;
            if let Some(existing) = self.logs.get_mut(pos) {
                existing.duplicate_count += 1;
                existing.metadata.timestamp = log.metadata.timestamp; // Update timestamp
                self.duplicates_detected += 1;
                return;
            }
        }
        
        // Add new log
        if !self.filter.should_include(&log) {
            return; // Filter out unwanted logs
        }
        
        self.logs.push_back(log);
        
        // Enforce capacity limit
        while self.logs.len() > self.capacity {
            self.logs.pop_front();
            self.logs_dropped += 1;
        }
        
        // Periodic cleanup of old logs (every 30 seconds)
        if let Ok(elapsed) = self.last_cleanup.elapsed() {
            if elapsed > Duration::from_secs(30) {
                self.cleanup_old_logs();
                self.last_cleanup = SystemTime::now();
            }
        }
    }
    
    fn get_logs(&self, filter: LogFilter, start: usize, count: usize) -> Vec<LogData> {
        let filtered_logs: Vec<_> = self.logs.iter()
            .filter(|log| filter.should_include(log))
            .collect();
        
        let end = std::cmp::min(start + count, filtered_logs.len());
        if start >= filtered_logs.len() {
            return Vec::new();
        }
        
        filtered_logs[start..end].iter().cloned().cloned().collect()
    }
    
    fn get_stats(&self) -> LogStoreStats {
        LogStoreStats {
            total_logs: self.total_logs,
            current_logs: self.logs.len(),
            duplicates_detected: self.duplicates_detected,
            capacity: self.capacity,
            logs_dropped: self.logs_dropped,
        }
    }
    
    fn update_filter(&mut self, config: FilterConfig) {
        self.filter.update_config(config);
    }
    
    fn clear(&mut self) {
        self.logs.clear();
        self.logs_dropped = 0;
        // Keep total_logs and duplicates_detected for lifetime stats
    }
    
    fn cleanup_old_logs(&mut self) {
        let cutoff = SystemTime::now() - Duration::from_secs(300); // 5 minutes
        let initial_len = self.logs.len();
        
        self.logs.retain(|log| {
            log.metadata.timestamp > cutoff || log.level >= LogLevel::Warn
        });
        
        let removed = initial_len - self.logs.len();
        if removed > 0 {
            self.logs_dropped += removed as u64;
        }
    }
}

impl CentralLogStore {
    /// Create a new central log store with the given capacity
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = bounded::<LogStoreMessage>(1000);
        
        let worker_handle = thread::spawn(move || {
            let mut worker = LogStoreWorker::new(capacity);
            
            loop {
                match receiver.recv() {
                    Ok(LogStoreMessage::AddLog(log)) => {
                        worker.add_log(log);
                    }
                    Ok(LogStoreMessage::GetLogs { filter, start, count, response }) => {
                        let logs = worker.get_logs(filter, start, count);
                        let _ = response.send(logs);
                    }
                    Ok(LogStoreMessage::GetStats { response }) => {
                        let stats = worker.get_stats();
                        let _ = response.send(stats);
                    }
                    Ok(LogStoreMessage::UpdateFilter(config)) => {
                        worker.update_filter(config);
                    }
                    Ok(LogStoreMessage::Clear) => {
                        worker.clear();
                    }
                    Ok(LogStoreMessage::Shutdown) => {
                        break;
                    }
                    Err(_) => {
                        // Channel closed, shutdown
                        break;
                    }
                }
            }
        });
        
        Self {
            sender,
            _worker_handle: worker_handle,
        }
    }
    
    /// Add a log entry to the store
    pub fn add_log(&self, log: LogData) {
        let _ = self.sender.try_send(LogStoreMessage::AddLog(log));
    }
    
    /// Add a simple log message
    pub fn add_message(&self, level: LogLevel, target: String, message: String) {
        let metadata = LogMetadata::new(target);
        let log = LogData::new(0, level, message, metadata); // ID will be set by worker
        self.add_log(log);
    }
    
    /// Get logs with filtering and pagination
    pub fn get_logs(&self, filter: LogFilter, start: usize, count: usize) -> Vec<LogData> {
        let (response_tx, response_rx) = bounded(1);
        
        if self.sender.send(LogStoreMessage::GetLogs {
            filter,
            start,
            count,
            response: response_tx,
        }).is_ok() {
            response_rx.recv().unwrap_or_default()
        } else {
            Vec::new()
        }
    }
    
    /// Get all logs (no filtering)
    pub fn get_all_logs(&self) -> Vec<LogData> {
        self.get_logs(LogFilter::default(), 0, usize::MAX)
    }
    
    /// Get recent logs (last N entries)
    pub fn get_recent_logs(&self, count: usize) -> Vec<LogData> {
        let all_logs = self.get_all_logs();
        let start = all_logs.len().saturating_sub(count);
        all_logs.into_iter().skip(start).collect()
    }
    
    /// Get logs by level
    pub fn get_logs_by_level(&self, level: LogLevel) -> Vec<LogData> {
        self.get_logs(LogFilter::for_level(level), 0, usize::MAX)
    }
    
    /// Get statistics about the log store
    pub fn get_stats(&self) -> LogStoreStats {
        let (response_tx, response_rx) = bounded(1);
        
        if self.sender.send(LogStoreMessage::GetStats {
            response: response_tx,
        }).is_ok() {
            response_rx.recv().unwrap_or(LogStoreStats {
                total_logs: 0,
                current_logs: 0,
                duplicates_detected: 0,
                capacity: 0,
                logs_dropped: 0,
            })
        } else {
            LogStoreStats {
                total_logs: 0,
                current_logs: 0,
                duplicates_detected: 0,
                capacity: 0,
                logs_dropped: 0,
            }
        }
    }
    
    /// Update the filter configuration
    pub fn update_filter(&self, config: FilterConfig) {
        let _ = self.sender.try_send(LogStoreMessage::UpdateFilter(config));
    }
    
    /// Clear all logs
    pub fn clear(&self) {
        let _ = self.sender.try_send(LogStoreMessage::Clear);
    }
}

impl Drop for CentralLogStore {
    fn drop(&mut self) {
        let _ = self.sender.try_send(LogStoreMessage::Shutdown);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    
    #[test]
    fn test_basic_log_storage() {
        let store = CentralLogStore::new(100);
        
        let metadata = LogMetadata::new("test".to_string());
        let log = LogData::new(1, LogLevel::Info, "Test message".to_string(), metadata);
        
        store.add_log(log.clone());
        thread::sleep(Duration::from_millis(10)); // Let worker process
        
        let logs = store.get_all_logs();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].message, "Test message");
    }
    
    #[test]
    fn test_capacity_limit() {
        let store = CentralLogStore::new(2);
        
        for i in 0..5 {
            let metadata = LogMetadata::new("test".to_string());
            let log = LogData::new(i, LogLevel::Info, format!("Message {}", i), metadata);
            store.add_log(log);
        }
        
        thread::sleep(Duration::from_millis(50)); // Let worker process
        
        let logs = store.get_all_logs();
        assert!(logs.len() <= 2);
        
        let stats = store.get_stats();
        assert_eq!(stats.total_logs, 5);
        assert!(stats.logs_dropped > 0);
    }
    
    #[test]
    fn test_duplicate_detection() {
        let store = CentralLogStore::new(100);
        
        let metadata = LogMetadata::new("test".to_string())
            .with_location("test.rs".to_string(), 42);
        
        // Add same log twice
        for _ in 0..2 {
            let log = LogData::new(1, LogLevel::Info, "Duplicate message".to_string(), metadata.clone());
            store.add_log(log);
        }
        
        thread::sleep(Duration::from_millis(50)); // Let worker process
        
        let logs = store.get_all_logs();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].duplicate_count, 1);
        
        let stats = store.get_stats();
        assert_eq!(stats.duplicates_detected, 1);
    }
}