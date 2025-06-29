use std::collections::VecDeque;
use bevy_ecs::prelude::*;
use bevy_log::{debug, warn};

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
    /// Frame counter for context
    frame_count: u64,
}

impl Default for DebugRingBuffer {
    fn default() -> Self {
        Self::new(100) // Default buffer size of 100 entries per category
    }
}

impl DebugRingBuffer {
    /// Create a new debug ring buffer with specified capacity
    pub fn new(max_size: usize) -> Self {
        Self {
            rendering_buffer: VecDeque::with_capacity(max_size),
            layout_buffer: VecDeque::with_capacity(max_size),
            general_buffer: VecDeque::with_capacity(max_size),
            max_size,
            frame_count: 0,
        }
    }

    /// Increment frame counter (call once per frame)
    pub fn next_frame(&mut self) {
        self.frame_count += 1;
    }

    /// Add a rendering debug message to the buffer
    pub fn add_rendering_context(&mut self, message: String) {
        let timestamped_message = format!("[Frame {}] {}", self.frame_count, message);
        
        // Add to buffer, removing oldest if at capacity
        if self.rendering_buffer.len() >= self.max_size {
            self.rendering_buffer.pop_front();
        }
        self.rendering_buffer.push_back(timestamped_message);
    }

    /// Add a layout debug message to the buffer
    pub fn add_layout_context(&mut self, message: String) {
        let timestamped_message = format!("[Frame {}] {}", self.frame_count, message);
        
        // Add to buffer, removing oldest if at capacity
        if self.layout_buffer.len() >= self.max_size {
            self.layout_buffer.pop_front();
        }
        self.layout_buffer.push_back(timestamped_message);
    }

    /// Add a general debug message to the buffer
    pub fn add_general_context(&mut self, message: String) {
        let timestamped_message = format!("[Frame {}] {}", self.frame_count, message);
        
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
                debug!("[DebugRingBuffer] Recent rendering context:");
                for msg in self.get_rendering_context(Some(20)) {
                    debug!("  {}", msg);
                }
            }
            "layout" => {
                debug!("[DebugRingBuffer] Recent layout context:");
                for msg in self.get_layout_context(Some(20)) {
                    debug!("  {}", msg);
                }
            }
            "all" => {
                self.dump_context("rendering");
                self.dump_context("layout");
                debug!("[DebugRingBuffer] Recent general context:");
                for msg in self.get_general_context(Some(20)) {
                    debug!("  {}", msg);
                }
            }
            _ => {
                warn!("[DebugRingBuffer] Unknown category: {}", category);
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
            frame_count: self.frame_count,
        }
    }

    /// Clear all buffers (useful for testing or memory management)
    pub fn clear_all(&mut self) {
        self.rendering_buffer.clear();
        self.layout_buffer.clear();
        self.general_buffer.clear();
        
        #[cfg(feature = "debug_logging")]
        debug!("[DebugRingBuffer] Cleared all debug context buffers");
    }
}

/// Statistics about the debug ring buffer state
#[derive(Debug)]
pub struct BufferStats {
    pub rendering_count: usize,
    pub layout_count: usize,
    pub general_count: usize,
    pub max_size: usize,
    pub frame_count: u64,
}

/// System to update the frame counter each frame
pub fn update_debug_ring_buffer_system(
    mut debug_buffer: ResMut<DebugRingBuffer>,
) {
    debug_buffer.next_frame();
}

/// System to periodically log buffer statistics (only with debug_logging feature)
#[cfg(feature = "debug_logging")]
pub fn log_buffer_stats_system(
    debug_buffer: Res<DebugRingBuffer>,
) {
    // Log stats every 300 frames (5 seconds at 60fps)
    let stats = debug_buffer.get_stats();
    if stats.frame_count % 300 == 0 {
        debug!("[DebugRingBuffer] Stats - Rendering: {}/{}, Layout: {}/{}, General: {}/{}, Frame: {}", 
            stats.rendering_count, stats.max_size,
            stats.layout_count, stats.max_size,
            stats.general_count, stats.max_size,
            stats.frame_count
        );
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