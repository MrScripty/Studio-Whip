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
    
    /// Central log store usage statistics (percentage full)
    pub const LOG_STORE_USAGE: DiagnosticId = DiagnosticId::from_u128(102);
    
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
                Diagnostic::new(UiDiagnosticsId::LOG_STORE_USAGE, "log_store_usage_percent", 120)
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
) {
    // Update UI node count
    diagnostics.add_measurement(&UiDiagnosticsId::UI_NODES_COUNT, timer.ui_nodes_processed as f64);
    
    // Update text layout count  
    diagnostics.add_measurement(&UiDiagnosticsId::TEXT_LAYOUT_COUNT, timer.text_layouts_processed as f64);
    
    // Update log store usage if available
    if let Some(log_store) = crate::logging::get_log_store() {
        let stats = log_store.get_stats();
        let usage_percent = if stats.capacity > 0 {
            (stats.current_logs as f64 / stats.capacity as f64) * 100.0
        } else {
            0.0
        };
        
        diagnostics.add_measurement(&UiDiagnosticsId::LOG_STORE_USAGE, usage_percent);
        
        // Also log performance metrics to our central log store
        tracing::debug!(
            target: "whip_ui::diagnostics::performance",
            ui_nodes_processed = timer.ui_nodes_processed,
            text_layouts_processed = timer.text_layouts_processed,
            log_store_usage_percent = usage_percent,
            "UI performance metrics"
        );
    }
}

/// System that resets frame counters after diagnostics update
fn ui_diagnostics_reset_system(
    mut timer: ResMut<UiDiagnosticsTimer>,
) {
    timer.reset_frame_counters();
}

/// Enhanced system that periodically logs comprehensive UI diagnostics using tracing
#[cfg(feature = "debug_logging")]
pub fn ui_diagnostics_log_system(
    diagnostics: Res<Diagnostics>,
    time: Option<Res<bevy_time::Time>>,
) {
    // Use Bevy's time system if available, otherwise fall back to system time
    let should_log = if let Some(time) = time {
        // Log every 5 seconds using Bevy's time system
        time.elapsed_seconds() as u64 % 5 == 0 && time.delta_seconds() < 0.1
    } else {
        // Fallback to system time
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        current_time % 5 == 0
    };
    
    if should_log {
        // Create a structured diagnostic summary
        let mut fps = None;
        let mut ui_nodes = None;
        let mut text_layouts = None;
        let mut log_store_usage = None;
        
        // Gather diagnostic data
        if let Some(fps_diag) = diagnostics.get(&bevy_diagnostic::FrameTimeDiagnosticsPlugin::FPS) {
            fps = fps_diag.smoothed();
        }
        
        if let Some(nodes_diag) = diagnostics.get(&UiDiagnosticsId::UI_NODES_COUNT) {
            ui_nodes = nodes_diag.smoothed();
        }
        
        if let Some(text_diag) = diagnostics.get(&UiDiagnosticsId::TEXT_LAYOUT_COUNT) {
            text_layouts = text_diag.smoothed();
        }
        
        if let Some(log_diag) = diagnostics.get(&UiDiagnosticsId::LOG_STORE_USAGE) {
            log_store_usage = log_diag.smoothed();
        }
        
        // Log comprehensive performance summary using structured tracing
        tracing::info!(
            target: "whip_ui::diagnostics::summary",
            fps = fps,
            ui_nodes_processed = ui_nodes.map(|n| n as u32),
            text_layouts_processed = text_layouts.map(|n| n as u32),
            log_store_usage_percent = log_store_usage,
            "UI Performance Summary"
        );
        
        // Also log to CentralLogStore if available for CLI access
        if let Some(log_store) = crate::logging::get_log_store() {
            let stats = log_store.get_stats();
            tracing::info!(
                target: "whip_ui::diagnostics::log_store",
                total_logs = stats.total_logs,
                current_logs = stats.current_logs,
                duplicates_detected = stats.duplicates_detected,
                logs_dropped = stats.logs_dropped,
                capacity = stats.capacity,
                "Log Store Statistics"
            );
        }
    }
}