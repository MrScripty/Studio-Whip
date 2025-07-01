//! Filtering system for logs

use crate::logging::types::{LogData, LogLevel};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Configuration for log filtering
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilterConfig {
    /// Minimum log level to include
    pub min_level: LogLevel,
    /// Specific targets to include (empty = all)
    pub include_targets: HashSet<String>,
    /// Specific targets to exclude
    pub exclude_targets: HashSet<String>,
    /// Specific categories to include (empty = all)
    pub include_categories: HashSet<String>,
    /// Specific categories to exclude
    pub exclude_categories: HashSet<String>,
    /// Maximum number of logs to keep in memory
    pub max_logs: usize,
    /// Simple target filter (contains match)
    pub target_filter: Option<String>,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            min_level: LogLevel::Debug,
            include_targets: HashSet::new(),
            exclude_targets: HashSet::new(),
            include_categories: HashSet::new(),
            exclude_categories: HashSet::new(),
            max_logs: 10000,
            target_filter: None,
        }
    }
}

/// Log filter for determining which logs to keep and display
#[derive(Debug, Clone, PartialEq)]
pub struct LogFilter {
    config: FilterConfig,
    exact_level: Option<LogLevel>,
}

impl LogFilter {
    /// Create a new filter with the given configuration
    pub fn new(config: FilterConfig) -> Self {
        Self { config, exact_level: None }
    }
    
    /// Create a filter with default configuration
    pub fn default() -> Self {
        Self::new(FilterConfig::default())
    }
    
    /// Check if a log passes the filter
    pub fn should_include(&self, log: &LogData) -> bool {
        // Check exact level first if specified
        if let Some(exact_level) = self.exact_level {
            if log.level != exact_level {
                return false;
            }
        } else {
            // Check minimum level
            if log.level < self.config.min_level {
                return false;
            }
        }
        
        // Check simple target filter first
        if let Some(ref filter) = self.config.target_filter {
            if !log.metadata.target.contains(filter) {
                return false;
            }
        }
        
        // Check target inclusion/exclusion
        if !self.config.include_targets.is_empty() 
            && !self.config.include_targets.contains(&log.metadata.target) {
            return false;
        }
        
        if self.config.exclude_targets.contains(&log.metadata.target) {
            return false;
        }
        
        // Check category inclusion/exclusion
        if let Some(ref category) = log.metadata.category {
            if !self.config.include_categories.is_empty() 
                && !self.config.include_categories.contains(category) {
                return false;
            }
            
            if self.config.exclude_categories.contains(category) {
                return false;
            }
        }
        
        true
    }
    
    /// Update the filter configuration
    pub fn update_config(&mut self, config: FilterConfig) {
        self.config = config;
    }
    
    /// Get the current configuration
    pub fn config(&self) -> &FilterConfig {
        &self.config
    }
    
    /// Set minimum log level
    pub fn set_min_level(&mut self, level: LogLevel) {
        self.config.min_level = level;
    }
    
    /// Add a target to include list
    pub fn include_target(&mut self, target: String) {
        self.config.include_targets.insert(target);
    }
    
    /// Add a target to exclude list
    pub fn exclude_target(&mut self, target: String) {
        self.config.exclude_targets.insert(target);
    }
    
    /// Add a category to include list
    pub fn include_category(&mut self, category: String) {
        self.config.include_categories.insert(category);
    }
    
    /// Add a category to exclude list
    pub fn exclude_category(&mut self, category: String) {
        self.config.exclude_categories.insert(category);
    }
    
    /// Clear all filters (show everything above min level)
    pub fn clear_filters(&mut self) {
        self.config.include_targets.clear();
        self.config.exclude_targets.clear();
        self.config.include_categories.clear();
        self.config.exclude_categories.clear();
    }
    
    /// Create a filter for a specific level (minimum level)
    pub fn for_level(level: LogLevel) -> Self {
        Self::new(FilterConfig {
            min_level: level,
            ..Default::default()
        })
    }
    
    /// Create a filter for exactly this level only
    pub fn for_exact_level(level: LogLevel) -> Self {
        let mut filter = Self::new(FilterConfig {
            min_level: LogLevel::Trace, // Allow all levels, we'll filter in should_include
            ..Default::default()
        });
        filter.exact_level = Some(level);
        filter
    }
    
    /// Create a filter for specific targets
    pub fn for_targets(targets: Vec<String>) -> Self {
        Self::new(FilterConfig {
            include_targets: targets.into_iter().collect(),
            ..Default::default()
        })
    }
    
    /// Create a filter for specific categories
    pub fn for_categories(categories: Vec<String>) -> Self {
        Self::new(FilterConfig {
            include_categories: categories.into_iter().collect(),
            ..Default::default()
        })
    }
}

impl Default for LogFilter {
    fn default() -> Self {
        Self::new(FilterConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::types::{LogMetadata, LogData};
    
    #[test]
    fn test_level_filtering() {
        let mut filter = LogFilter::for_level(LogLevel::Warn);
        
        let meta = LogMetadata::new("test".to_string());
        let debug_log = LogData::new(1, LogLevel::Debug, "Debug".to_string(), meta.clone());
        let warn_log = LogData::new(2, LogLevel::Warn, "Warn".to_string(), meta.clone());
        let error_log = LogData::new(3, LogLevel::Error, "Error".to_string(), meta);
        
        assert!(!filter.should_include(&debug_log));
        assert!(filter.should_include(&warn_log));
        assert!(filter.should_include(&error_log));
    }
    
    #[test]
    fn test_target_filtering() {
        let filter = LogFilter::for_targets(vec!["whip_ui::rendering".to_string()]);
        
        let meta1 = LogMetadata::new("whip_ui::rendering".to_string());
        let meta2 = LogMetadata::new("whip_ui::layout".to_string());
        
        let render_log = LogData::new(1, LogLevel::Info, "Render".to_string(), meta1);
        let layout_log = LogData::new(2, LogLevel::Info, "Layout".to_string(), meta2);
        
        assert!(filter.should_include(&render_log));
        assert!(!filter.should_include(&layout_log));
    }
    
    #[test]
    fn test_category_filtering() {
        let filter = LogFilter::for_categories(vec!["performance".to_string()]);
        
        let meta1 = LogMetadata::new("test".to_string())
            .with_category("performance".to_string());
        let meta2 = LogMetadata::new("test".to_string())
            .with_category("debug".to_string());
        
        let perf_log = LogData::new(1, LogLevel::Info, "Performance".to_string(), meta1);
        let debug_log = LogData::new(2, LogLevel::Info, "Debug".to_string(), meta2);
        
        assert!(filter.should_include(&perf_log));
        assert!(!filter.should_include(&debug_log));
    }
}