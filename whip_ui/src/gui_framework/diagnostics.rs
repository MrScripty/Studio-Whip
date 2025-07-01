use bevy_diagnostic::{Diagnostic, DiagnosticId, Diagnostics, RegisterDiagnostic};
use bevy_ecs::prelude::*;
use std::time::{Duration, Instant};

/// Diagnostic IDs for UI-specific metrics
pub struct UiDiagnosticsId;

impl UiDiagnosticsId {
    /// Layout computation time per frame (in milliseconds)
    pub const LAYOUT_COMPUTATION_TIME: DiagnosticId = DiagnosticId::from_u128(100);
    
    /// State tracking overhead per frame (in milliseconds)  
    pub const STATE_TRACKING_TIME: DiagnosticId = DiagnosticId::from_u128(101);
    
    /// Ring buffer usage statistics (percentage full)
    pub const RING_BUFFER_USAGE: DiagnosticId = DiagnosticId::from_u128(102);
    
    /// Number of UI nodes processed per frame
    pub const UI_NODES_COUNT: DiagnosticId = DiagnosticId::from_u128(103);
    
    /// Text layout operations per frame
    pub const TEXT_LAYOUT_COUNT: DiagnosticId = DiagnosticId::from_u128(104);
}

/// Resource to track timing for UI operations
#[derive(Resource, Default)]
pub struct UiDiagnosticsTimer {
    /// Layout computation start time
    pub layout_start: Option<Instant>,
    /// State tracking start time
    pub state_tracking_start: Option<Instant>,
    /// Frame counters for operations
    pub ui_nodes_processed: u32,
    pub text_layouts_processed: u32,
}

impl UiDiagnosticsTimer {
    /// Start timing layout computation
    pub fn start_layout_timing(&mut self) {
        self.layout_start = Some(Instant::now());
    }
    
    /// End timing layout computation and return duration
    pub fn end_layout_timing(&mut self) -> Option<Duration> {
        self.layout_start.take().map(|start| start.elapsed())
    }
    
    /// Start timing state tracking
    pub fn start_state_tracking_timing(&mut self) {
        self.state_tracking_start = Some(Instant::now());
    }
    
    /// End timing state tracking and return duration
    pub fn end_state_tracking_timing(&mut self) -> Option<Duration> {
        self.state_tracking_start.take().map(|start| start.elapsed())
    }
    
    /// Increment UI nodes processed counter
    pub fn increment_ui_nodes(&mut self, count: u32) {
        self.ui_nodes_processed = self.ui_nodes_processed.saturating_add(count);
    }
    
    /// Increment text layout operations counter
    pub fn increment_text_layouts(&mut self, count: u32) {
        self.text_layouts_processed = self.text_layouts_processed.saturating_add(count);
    }
    
    /// Reset frame counters (call at end of frame)
    pub fn reset_frame_counters(&mut self) {
        self.ui_nodes_processed = 0;
        self.text_layouts_processed = 0;
    }
}

/// Plugin that registers UI diagnostics
pub struct UiDiagnosticsPlugin;

impl bevy_app::Plugin for UiDiagnosticsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<UiDiagnosticsTimer>()
            .register_diagnostic(
                Diagnostic::new(UiDiagnosticsId::LAYOUT_COMPUTATION_TIME, "layout_computation_time_ms", 120)
                    .with_suffix(" ms")
            )
            .register_diagnostic(
                Diagnostic::new(UiDiagnosticsId::STATE_TRACKING_TIME, "state_tracking_time_ms", 120)
                    .with_suffix(" ms")
            )
            .register_diagnostic(
                Diagnostic::new(UiDiagnosticsId::RING_BUFFER_USAGE, "ring_buffer_usage_percent", 120)
                    .with_suffix("%")
            )
            .register_diagnostic(
                Diagnostic::new(UiDiagnosticsId::UI_NODES_COUNT, "ui_nodes_count", 120)
                    .with_suffix(" nodes")
            )
            .register_diagnostic(
                Diagnostic::new(UiDiagnosticsId::TEXT_LAYOUT_COUNT, "text_layout_count", 120)
                    .with_suffix(" layouts")
            )
            .add_systems(bevy_app::Last, (
                ui_diagnostics_update_system,
                ui_diagnostics_reset_system.after(ui_diagnostics_update_system),
            ));
    }
}

/// System that updates UI diagnostics with current frame data
fn ui_diagnostics_update_system(
    mut diagnostics: ResMut<Diagnostics>,
    timer: Res<UiDiagnosticsTimer>,
    debug_buffer: Option<Res<crate::gui_framework::debug::DebugRingBuffer>>,
) {
    // Update UI node count
    diagnostics.add_measurement(&UiDiagnosticsId::UI_NODES_COUNT, timer.ui_nodes_processed as f64);
    
    // Update text layout count  
    diagnostics.add_measurement(&UiDiagnosticsId::TEXT_LAYOUT_COUNT, timer.text_layouts_processed as f64);
    
    // Update ring buffer usage if available
    if let Some(buffer) = debug_buffer {
        let stats = buffer.get_stats();
        let total_used = stats.rendering_count + stats.layout_count + stats.general_count;
        let total_capacity = stats.max_size * 3; // 3 buffers
        let usage_percent = if total_capacity > 0 {
            (total_used as f64 / total_capacity as f64) * 100.0
        } else {
            0.0
        };
        
        diagnostics.add_measurement(&UiDiagnosticsId::RING_BUFFER_USAGE, usage_percent);
    }
}

/// System that resets frame counters after diagnostics update
fn ui_diagnostics_reset_system(
    mut timer: ResMut<UiDiagnosticsTimer>,
) {
    timer.reset_frame_counters();
}

/// System that logs UI diagnostics when debug_logging is enabled
#[cfg(feature = "debug_logging")]
pub fn ui_diagnostics_log_system(
    diagnostics: Res<Diagnostics>,
    mut debug_buffer: Option<ResMut<crate::gui_framework::debug::DebugRingBuffer>>,
) {
    use bevy_log::debug;
    use bevy_time::Time;
    
    // Simple periodic logging based on time instead of frame count
    // This avoids the frame count diagnostic path issue for now
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    // Log every 5 seconds
    if current_time % 5 == 0 {
        let mut diagnostic_messages = Vec::new();
        diagnostic_messages.push("[UI Diagnostics] Performance Summary:".to_string());
        
        // Frame time info
        if let Some(fps_diag) = diagnostics.get(&bevy_diagnostic::FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(fps) = fps_diag.smoothed() {
                diagnostic_messages.push(format!("  FPS: {:.1}", fps));
            }
        }
        
        // UI specific metrics
        if let Some(nodes_diag) = diagnostics.get(&UiDiagnosticsId::UI_NODES_COUNT) {
            if let Some(count) = nodes_diag.smoothed() {
                diagnostic_messages.push(format!("  UI nodes processed: {:.0}", count));
            }
        }
        
        if let Some(text_diag) = diagnostics.get(&UiDiagnosticsId::TEXT_LAYOUT_COUNT) {
            if let Some(count) = text_diag.smoothed() {
                diagnostic_messages.push(format!("  Text layouts: {:.0}", count));
            }
        }
        
        if let Some(buffer_diag) = diagnostics.get(&UiDiagnosticsId::RING_BUFFER_USAGE) {
            if let Some(usage) = buffer_diag.smoothed() {
                diagnostic_messages.push(format!("  Ring buffer usage: {:.1}%", usage));
            }
        }
        
        // Output to ring buffer or log
        if let Some(ref mut buffer) = debug_buffer {
            for message in diagnostic_messages {
                buffer.add_general_context(message);
            }
        } else {
            for message in diagnostic_messages {
                debug!("{}", message);
            }
        }
    }
}