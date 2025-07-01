//! Custom tracing layer that forwards events to CentralLogStore

use crate::logging::{get_log_store, LogData, LogLevel, LogMetadata};
use tracing::{field::Visit, Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

/// Custom tracing layer that captures events and forwards them to CentralLogStore
#[derive(Debug)]
pub struct WhipUiTracingLayer;

impl WhipUiTracingLayer {
    /// Create a new WhipUI tracing layer
    pub fn new() -> Self {
        Self
    }
}

/// Field visitor that collects field values into a string
#[derive(Default)]
struct FieldCollector {
    message: String,
    fields: Vec<(String, String)>,
}

impl Visit for FieldCollector {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
            // Remove quotes from debug format if it's a simple string
            if self.message.starts_with('"') && self.message.ends_with('"') && self.message.len() > 2 {
                self.message = self.message[1..self.message.len()-1].to_string();
            }
        } else {
            self.fields.push((field.name().to_string(), format!("{:?}", value)));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields.push((field.name().to_string(), value.to_string()));
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields.push((field.name().to_string(), value.to_string()));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields.push((field.name().to_string(), value.to_string()));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.fields.push((field.name().to_string(), value.to_string()));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields.push((field.name().to_string(), value.to_string()));
    }
}

impl<S> Layer<S> for WhipUiTracingLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // Get the log store, if not initialized just return
        let log_store = match get_log_store() {
            Some(store) => store,
            None => {
                // Logging service not initialized, skip
                return;
            }
        };

        // Extract metadata from the tracing event
        let metadata = event.metadata();
        let level = LogLevel::from_tracing(metadata.level());
        let target = metadata.target().to_string();

        // Collect all field values
        let mut collector = FieldCollector::default();
        event.record(&mut collector);

        // Build the final message
        let mut final_message = collector.message;
        if !collector.fields.is_empty() {
            if !final_message.is_empty() {
                final_message.push(' ');
            }
            
            // Append structured fields
            let fields_str: Vec<String> = collector.fields
                .iter()
                .map(|(key, value)| format!("{}={}", key, value))
                .collect();
            
            if !fields_str.is_empty() {
                final_message.push_str(&format!("[{}]", fields_str.join(", ")));
            }
        }

        // Create log metadata
        let mut log_metadata = LogMetadata::new(target);
        
        // Add file and line information if available
        if let Some(file) = metadata.file() {
            if let Some(line) = metadata.line() {
                log_metadata = log_metadata.with_location(file.to_string(), line);
            }
        }

        // Determine category based on target module path
        let category = extract_category_from_target(&log_metadata.target);
        if let Some(cat) = category {
            log_metadata = log_metadata.with_category(cat);
        }

        // Create and add the log entry
        let log_data = LogData::new(0, level, final_message, log_metadata); // ID will be set by worker
        log_store.add_log(log_data);
    }
}

/// Extract a category from the target module path for easier filtering
fn extract_category_from_target(target: &str) -> Option<String> {
    // Map common module patterns to categories
    if target.contains("render") {
        Some("rendering".to_string())
    } else if target.contains("layout") {
        Some("layout".to_string())
    } else if target.contains("input") || target.contains("interaction") {
        Some("input".to_string())
    } else if target.contains("asset") {
        Some("assets".to_string())
    } else if target.contains("widget") {
        Some("widgets".to_string())
    } else if target.contains("vulkan") || target.contains("gpu") {
        Some("graphics".to_string())
    } else if target.contains("bevy") {
        Some("ecs".to_string())
    } else {
        // For whip_ui modules, use the last component as category
        let parts: Vec<&str> = target.split("::").collect();
        if parts.len() > 1 && target.starts_with("whip_ui") {
            Some(parts.last().unwrap_or(&"general").to_string())
        } else {
            Some("general".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::init_logging_service;
    use tracing_subscriber::prelude::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_tracing_integration() {
        // Initialize logging service
        init_logging_service(100).unwrap();

        // Set up tracing with our custom layer
        let layer = WhipUiTracingLayer::new();
        let subscriber = tracing_subscriber::registry()
            .with(layer);

        tracing::subscriber::with_default(subscriber, || {
            // Generate some test logs
            tracing::info!("Test info message");
            tracing::warn!(value = 42, "Test warning with field");
            tracing::error!("Test error message");
        });

        // Give the background worker time to process
        thread::sleep(Duration::from_millis(50));

        // Check that logs were captured
        let store = get_log_store().unwrap();
        let logs = store.get_all_logs();
        
        assert!(!logs.is_empty(), "No logs were captured");
        assert!(logs.iter().any(|log| log.message.contains("Test info message")));
        assert!(logs.iter().any(|log| log.message.contains("Test warning with field")));
        assert!(logs.iter().any(|log| log.message.contains("Test error message")));
    }

    #[test]
    fn test_category_extraction() {
        assert_eq!(extract_category_from_target("whip_ui::rendering::engine"), Some("rendering".to_string()));
        assert_eq!(extract_category_from_target("whip_ui::layout::systems"), Some("layout".to_string()));
        assert_eq!(extract_category_from_target("whip_ui::widgets::button"), Some("widgets".to_string()));
        assert_eq!(extract_category_from_target("some_external_crate"), Some("general".to_string()));
    }
}