use std::collections::VecDeque;
use std::time::{Duration, Instant};
use bevy_ecs::prelude::*;

/// A ring buffer for capturing debug context to prevent log overflow
/// while preserving recent debugging information for analysis
#[derive(Resource)]
pub struct DebugRingBuffer {
    /// Buffer for rendering context messages
    rendering_buffer: VecDeque<String>,
    /// Buffer for layout context messages  
    layout_buffer: VecDeque<String>,
    /// Buffer for general debug messages
    general_buffer: VecDeque<String>,
    /// Maximum size for each buffer
    max_size: usize,
    /// Creation time for calculating elapsed time
    created_at: Instant,
    /// Last stats log time
    last_stats_log: Instant,
}

impl Default for DebugRingBuffer {
    fn default() -> Self {
        Self::new(100) // Default buffer size of 100 entries per category
    }
}

impl DebugRingBuffer {
    /// Create a new debug ring buffer with specified capacity
    pub fn new(max_size: usize) -> Self {
        let now = Instant::now();
        Self {
            rendering_buffer: VecDeque::with_capacity(max_size),
            layout_buffer: VecDeque::with_capacity(max_size),
            general_buffer: VecDeque::with_capacity(max_size),
            max_size,
            created_at: now,
            last_stats_log: now,
        }
    }

    /// Update the last stats log time (called periodically)
    pub fn update_stats_log_time(&mut self) {
        self.last_stats_log = Instant::now();
    }

    /// Check if it's time to log stats (every 5 seconds)
    pub fn should_log_stats(&self) -> bool {
        self.last_stats_log.elapsed() >= Duration::from_secs(5)
    }

    /// Add a rendering debug message to the buffer
    pub fn add_rendering_context(&mut self, message: String) {
        let elapsed_secs = self.created_at.elapsed().as_secs_f32();
        let timestamped_message = format!("[{:.2}s] {}", elapsed_secs, message);
        
        // Add to buffer, removing oldest if at capacity
        if self.rendering_buffer.len() >= self.max_size {
            self.rendering_buffer.pop_front();
        }
        self.rendering_buffer.push_back(timestamped_message);
    }

    /// Add a layout debug message to the buffer
    pub fn add_layout_context(&mut self, message: String) {
        let elapsed_secs = self.created_at.elapsed().as_secs_f32();
        let timestamped_message = format!("[{:.2}s] {}", elapsed_secs, message);
        
        // Add to buffer, removing oldest if at capacity
        if self.layout_buffer.len() >= self.max_size {
            self.layout_buffer.pop_front();
        }
        self.layout_buffer.push_back(timestamped_message);
    }

    /// Add a general debug message to the buffer
    pub fn add_general_context(&mut self, message: String) {
        let elapsed_secs = self.created_at.elapsed().as_secs_f32();
        let timestamped_message = format!("[{:.2}s] {}", elapsed_secs, message);
        
        // Add to buffer, removing oldest if at capacity
        if self.general_buffer.len() >= self.max_size {
            self.general_buffer.pop_front();
        }
        self.general_buffer.push_back(timestamped_message);
    }


    /// Get recent rendering context (last N messages)
    pub fn get_rendering_context(&self, count: Option<usize>) -> Vec<String> {
        let count = count.unwrap_or(10).min(self.rendering_buffer.len());
        self.rendering_buffer.iter()
            .rev()
            .take(count)
            .rev()
            .cloned()
            .collect()
    }

    /// Get recent layout context (last N messages)
    pub fn get_layout_context(&self, count: Option<usize>) -> Vec<String> {
        let count = count.unwrap_or(10).min(self.layout_buffer.len());
        self.layout_buffer.iter()
            .rev()
            .take(count)
            .rev()
            .cloned()
            .collect()
    }

    /// Get recent general context (last N messages)
    pub fn get_general_context(&self, count: Option<usize>) -> Vec<String> {
        let count = count.unwrap_or(10).min(self.general_buffer.len());
        self.general_buffer.iter()
            .rev()
            .take(count)
            .rev()
            .cloned()
            .collect()
    }

    /// Dump all context for debugging purposes (only when debug_logging enabled)
    #[cfg(feature = "debug_logging")]
    pub fn dump_context(&self, category: &str) {
        match category {
            "rendering" => {
                bevy_log::debug!("[DebugRingBuffer] Recent rendering context:");
                for msg in self.get_rendering_context(Some(20)) {
                    bevy_log::debug!("  {}", msg);
                }
            }
            "layout" => {
                bevy_log::debug!("[DebugRingBuffer] Recent layout context:");
                for msg in self.get_layout_context(Some(20)) {
                    bevy_log::debug!("  {}", msg);
                }
            }
            "all" => {
                self.dump_context("rendering");
                self.dump_context("layout");
                bevy_log::debug!("[DebugRingBuffer] Recent general context:");
                for msg in self.get_general_context(Some(20)) {
                    bevy_log::debug!("  {}", msg);
                }
            }
            _ => {
                bevy_log::warn!("[DebugRingBuffer] Unknown category: {}", category);
            }
        }
    }

    /// Get buffer statistics
    pub fn get_stats(&self) -> BufferStats {
        BufferStats {
            rendering_count: self.rendering_buffer.len(),
            layout_count: self.layout_buffer.len(),
            general_count: self.general_buffer.len(),
            max_size: self.max_size,
            elapsed_seconds: self.created_at.elapsed().as_secs_f32(),
        }
    }

    /// Clear all buffers (useful for testing or memory management)
    pub fn clear_all(&mut self) {
        self.rendering_buffer.clear();
        self.layout_buffer.clear();
        self.general_buffer.clear();
        
        #[cfg(feature = "debug_logging")]
        bevy_log::debug!("[DebugRingBuffer] Cleared all debug context buffers");
    }
}

/// Statistics about the debug ring buffer state
#[derive(Debug)]
pub struct BufferStats {
    pub rendering_count: usize,
    pub layout_count: usize,
    pub general_count: usize,
    pub max_size: usize,
    pub elapsed_seconds: f32,
}

/// System to update debug ring buffer state each frame
pub fn update_debug_ring_buffer_system(
    _debug_buffer: ResMut<DebugRingBuffer>,
) {
    // No longer need to increment frame count, just exists for compatibility
    // Could be used for future frame-based tracking if needed
}

/// System to periodically log buffer statistics (only with debug_logging feature)
#[cfg(feature = "debug_logging")]
pub fn log_buffer_stats_system(
    mut debug_buffer: ResMut<DebugRingBuffer>,
) {
    // Log stats every 5 seconds using time-based approach
    if debug_buffer.should_log_stats() {
        let stats = debug_buffer.get_stats();
        bevy_log::debug!("[DebugRingBuffer] Stats - Rendering: {}/{}, Layout: {}/{}, General: {}/{}, Elapsed: {:.1}s", 
            stats.rendering_count, stats.max_size,
            stats.layout_count, stats.max_size,
            stats.general_count, stats.max_size,
            stats.elapsed_seconds
        );
        debug_buffer.update_stats_log_time();
    }
}

/// Helper macro for adding rendering context
#[macro_export]
macro_rules! add_rendering_context {
    ($buffer:expr, $($arg:tt)*) => {
        #[cfg(feature = "debug_logging")]
        $buffer.add_rendering_context(format!($($arg)*));
    };
}

/// Helper macro for adding layout context  
#[macro_export]
macro_rules! add_layout_context {
    ($buffer:expr, $($arg:tt)*) => {
        #[cfg(feature = "debug_logging")]
        $buffer.add_layout_context(format!($($arg)*));
    };
}

/// Helper macro for adding general context
#[macro_export]
macro_rules! add_general_context {
    ($buffer:expr, $($arg:tt)*) => {
        #[cfg(feature = "debug_logging")]
        $buffer.add_general_context(format!($($arg)*));
    };
}