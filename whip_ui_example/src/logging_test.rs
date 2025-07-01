//! Test module for demonstrating the advanced logging service

use whip_ui::{init_logging_service, init_tracing, get_log_store, LogLevel};
use tracing::{info, warn, error, debug, trace};

/// Initialize the logging system for testing
pub fn init_test_logging() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logging service with a capacity of 1000 logs
    init_logging_service(1000)
        .map_err(|e| format!("Failed to initialize logging service: {}", e))?;
    
    // Initialize tracing with our custom layer
    init_tracing()?;
    
    info!("Logging system initialized successfully");
    Ok(())
}

/// Generate test logs to demonstrate the logging system
pub fn generate_test_logs() {
    info!("Starting logging system test");
    
    // Test different log levels
    trace!(component = "renderer", "TEST: Trace level message - very detailed");
    debug!(frame_time = 16.67, "TEST: Frame rendered in {}ms", 16.67);
    info!(user_action = "button_click", "TEST: User clicked the save button");
    warn!(memory_usage = 85, "TEST: Memory usage is getting high: {}%", 85);
    error!(error_code = 404, "TEST: Simulated asset load failure");
    
    // Test repeated messages (should be deduplicated)
    for i in 0..5 {
        info!("Repeated message {}", i);
        if i < 3 {
            info!("This will be deduplicated");
        }
    }
    
    // Test different modules/targets
    log_from_different_modules();
    
    info!("Logging test completed");
    
    // Wait a bit longer for the background worker to process all logs
    std::thread::sleep(std::time::Duration::from_millis(50));
}

/// Simulate logs from different modules to test categorization
fn log_from_different_modules() {
    // Simulate rendering logs
    tracing::info!(target: "whip_ui::rendering::vulkan", "TEST: Vulkan command buffer recorded");
    tracing::warn!(target: "whip_ui::rendering::buffer", gpu_memory = "512MB", "TEST: GPU memory usage high");
    
    // Simulate layout logs  
    tracing::debug!(target: "whip_ui::layout::taffy", "TEST: Layout tree updated");
    tracing::trace!(target: "whip_ui::layout::positioning", x = 100, y = 200, "TEST: Element positioned");
    
    // Simulate input logs
    tracing::info!(target: "whip_ui::input::events", "TEST: Mouse click detected");
    tracing::debug!(target: "whip_ui::interaction::hotkeys", key = "Ctrl+S", "TEST: Hotkey triggered");
    
    // Simulate widget logs
    tracing::info!(target: "whip_ui::widgets::button", id = "save_btn", "TEST: Button state changed");
    tracing::warn!(target: "whip_ui::widgets::text", "TEST: Text overflow detected");
    
    // Simulate asset logs
    tracing::info!(target: "whip_ui::assets::loader", file = "ui/main.toml", "TEST: UI asset loaded");
    tracing::error!(target: "whip_ui::assets::registry", "TEST: Simulated asset registry error");
}

/// Print statistics about the current log store
pub fn print_log_statistics() {
    if let Some(store) = get_log_store() {
        let stats = store.get_stats();
        
        println!("\n=== Log Store Statistics ===");
        println!("Total logs processed: {}", stats.total_logs);
        println!("Current logs in store: {}", stats.current_logs);
        println!("Duplicates detected: {}", stats.duplicates_detected);
        println!("Logs dropped (capacity): {}", stats.logs_dropped);
        println!("Store capacity: {}", stats.capacity);
        
        // Show recent logs by level
        println!("\n=== Recent Logs by Level ===");
        for level in [LogLevel::Error, LogLevel::Warn, LogLevel::Info, LogLevel::Debug, LogLevel::Trace] {
            let logs = store.get_logs_by_level(level);
            if !logs.is_empty() {
                println!("{}: {} logs", level, logs.len());
                // Show the most recent log of this level
                if let Some(recent) = logs.last() {
                    println!("  Latest: {}", recent.format_display());
                }
            }
        }
        
        // Show logs by category
        println!("\n=== All Recent Logs ===");
        let recent_logs = store.get_recent_logs(10);
        for log in recent_logs {
            println!("{}", log.format_display());
        }
    } else {
        println!("Logging system not initialized!");
    }
}

/// Test the filtering capabilities
pub fn test_filtering() {
    if let Some(store) = get_log_store() {
        println!("\n=== Testing Filters ===");
        
        // Test level filtering
        let error_logs = store.get_logs_by_level(LogLevel::Error);
        println!("Error logs: {}", error_logs.len());
        
        let warn_logs = store.get_logs_by_level(LogLevel::Warn);
        println!("Warning logs: {}", warn_logs.len());
        
        // Test with custom filter
        use whip_ui::{LogFilter, FilterConfig};
        use std::collections::HashSet;
        
        let mut include_targets = HashSet::new();
        include_targets.insert("whip_ui::rendering::vulkan".to_string());
        
        let filter_config = FilterConfig {
            min_level: LogLevel::Debug,
            include_targets,
            exclude_targets: HashSet::new(),
            include_categories: HashSet::new(),
            exclude_categories: HashSet::new(),
            max_logs: 1000,
        };
        
        let filter = LogFilter::new(filter_config);
        let filtered_logs = store.get_logs(filter, 0, 100);
        println!("Vulkan rendering logs: {}", filtered_logs.len());
        
        for log in filtered_logs.iter().take(3) {
            println!("  {}", log.format_display());
        }
    }
}