//! Debug utilities for GUI framework
//! 
//! This module provides debugging tools and utilities to help with development
//! and troubleshooting of the GUI framework, including ring buffers for context
//! tracking and performance monitoring.

pub mod ring_buffer;

pub use ring_buffer::{DebugRingBuffer, BufferStats, update_debug_ring_buffer_system};

#[cfg(feature = "debug_logging")]
pub use ring_buffer::log_buffer_stats_system;