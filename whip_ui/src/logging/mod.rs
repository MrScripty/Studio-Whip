//! Advanced logging service for whip_ui
//! 
//! This module implements a hybrid push/pull logging architecture with:
//! - Thread-safe central log storage
//! - Global filtering capabilities
//! - Integration with the tracing ecosystem
//! - Minimal performance overhead

pub mod filter;
pub mod store;
pub mod types;

pub use filter::{LogFilter, FilterConfig};
pub use store::CentralLogStore;
pub use types::{LogData, LogLevel, LogMetadata};

use once_cell::sync::OnceCell;
use std::sync::Arc;

/// Global instance of the central log store
static LOG_STORE: OnceCell<Arc<CentralLogStore>> = OnceCell::new();

/// Initialize the global logging service
pub fn init_logging_service(capacity: usize) -> Result<(), &'static str> {
    LOG_STORE.set(Arc::new(CentralLogStore::new(capacity)))
        .map_err(|_| "Logging service already initialized")
}

/// Get a reference to the global log store
pub fn get_log_store() -> Option<Arc<CentralLogStore>> {
    LOG_STORE.get().cloned()
}

/// Initialize tracing subscriber with our custom layer
pub fn init_tracing() -> Result<(), Box<dyn std::error::Error>> {
    use tracing_subscriber::prelude::*;
    
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);
    
    let subscriber = tracing_subscriber::registry()
        .with(fmt_layer)
        .with(tracing_subscriber::EnvFilter::from_default_env());
    
    tracing::subscriber::set_global_default(subscriber)?;
    
    Ok(())
}