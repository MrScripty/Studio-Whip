//! Terminal renderer trait and implementations

use crate::logging::{LogData, LogLevel};
use crossterm::{
    cursor::MoveTo,
    style::{self, Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
    ExecutableCommand, QueueableCommand,
};
use std::io::{self, Write};

/// Frame state passed to the renderer
#[derive(Debug)]
pub struct CliFrameState<'a> {
    /// Logs to display (already filtered)
    pub logs: &'a [LogData],
    /// Current input buffer content
    pub input_buffer: &'a str,
    /// Status message to display
    pub status_message: &'a str,
    /// Current scroll offset
    pub scroll_offset: usize,
    /// Terminal dimensions
    pub terminal_size: (u16, u16),
    /// Currently selected log index (for detail view)
    pub selected_index: Option<usize>,
}

/// Trait for terminal renderers
pub trait TerminalRenderer {
    /// Draw a frame to the terminal
    fn draw(&mut self, state: CliFrameState) -> Result<(), Box<dyn std::error::Error>>;
    
    /// Handle terminal resize
    fn resize(&mut self, width: u16, height: u16);
}

/// Basic terminal renderer using direct crossterm output
pub struct BasicTerminalRenderer {
    /// Cached terminal size
    terminal_size: (u16, u16),
}

impl BasicTerminalRenderer {
    /// Create a new basic terminal renderer
    pub fn new() -> Self {
        let size = terminal::size().unwrap_or((80, 24));
        Self {
            terminal_size: size,
        }
    }
    
    /// Get color for log level
    fn level_color(level: LogLevel) -> Color {
        match level {
            LogLevel::Trace => Color::DarkGrey,
            LogLevel::Debug => Color::Cyan,
            LogLevel::Info => Color::Green,
            LogLevel::Warn => Color::Yellow,
            LogLevel::Error => Color::Red,
        }
    }
    
    /// Format a log line for display
    fn format_log_line(&self, log: &LogData, width: u16, show_full: bool) -> String {
        let timestamp = log.metadata.timestamp
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Simple time formatting without chrono
        let hours = (timestamp / 3600) % 24;
        let minutes = (timestamp / 60) % 60;
        let seconds = timestamp % 60;
        let time_str = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
        
        let level_str = format!("{:5}", log.level.to_string());
        let target = &log.metadata.target;
        
        // Calculate available width for message
        let prefix_len = time_str.len() + level_str.len() + target.len() + 4; // spaces
        let msg_width = (width as usize).saturating_sub(prefix_len);
        
        let message = if show_full {
            &log.message
        } else {
            // Truncate message if needed
            if log.message.len() > msg_width {
                &log.message[..msg_width.saturating_sub(3)]
            } else {
                &log.message
            }
        };
        
        let dup_indicator = if log.duplicate_count > 0 {
            format!(" (x{})", log.duplicate_count + 1)
        } else {
            String::new()
        };
        
        format!("{} {} {} {}{}", time_str, level_str, target, message, dup_indicator)
    }
}

impl TerminalRenderer for BasicTerminalRenderer {
    fn draw(&mut self, state: CliFrameState) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();
        
        // Clear screen
        stdout.execute(Clear(ClearType::All))?;
        
        // Draw header
        stdout.queue(MoveTo(0, 0))?;
        stdout.queue(SetForegroundColor(Color::Blue))?;
        stdout.queue(Print("=== Whip UI Log Viewer ==="))?;
        stdout.queue(ResetColor)?;
        
        // Draw status line
        stdout.queue(MoveTo(0, 1))?;
        stdout.queue(Print(&state.status_message))?;
        
        // Calculate log display area
        let header_lines = 3; // header + status + separator
        let footer_lines = 3; // input line + separator + help
        let log_area_height = state.terminal_size.1.saturating_sub(header_lines + footer_lines);
        
        // Draw separator
        stdout.queue(MoveTo(0, 2))?;
        stdout.queue(Print("─".repeat(state.terminal_size.0 as usize)))?;
        
        // Draw logs
        let visible_logs = state.logs.iter()
            .skip(state.scroll_offset)
            .take(log_area_height as usize);
        
        let mut line_num = header_lines;
        for (idx, log) in visible_logs.enumerate() {
            stdout.queue(MoveTo(0, line_num))?;
            
            // Highlight selected log
            let is_selected = state.selected_index == Some(state.scroll_offset + idx);
            if is_selected {
                stdout.queue(SetForegroundColor(Color::Black))?;
                stdout.queue(style::SetBackgroundColor(Color::White))?;
            }
            
            // Set log level color
            if !is_selected {
                stdout.queue(SetForegroundColor(Self::level_color(log.level)))?;
            }
            
            // Format and print log line
            let log_line = self.format_log_line(log, state.terminal_size.0, is_selected);
            stdout.queue(Print(log_line))?;
            
            if is_selected {
                stdout.queue(style::ResetColor)?;
            }
            stdout.queue(ResetColor)?;
            
            line_num += 1;
        }
        
        // Draw footer separator
        let footer_start = state.terminal_size.1.saturating_sub(footer_lines);
        stdout.queue(MoveTo(0, footer_start))?;
        stdout.queue(Print("─".repeat(state.terminal_size.0 as usize)))?;
        
        // Draw input line
        stdout.queue(MoveTo(0, footer_start + 1))?;
        stdout.queue(Print("> "))?;
        stdout.queue(Print(&state.input_buffer))?;
        
        // Draw help line
        stdout.queue(MoveTo(0, footer_start + 2))?;
        stdout.queue(SetForegroundColor(Color::DarkGrey))?;
        stdout.queue(Print("Commands: /quit, /filter <level>, /clear, /save <path> | ↑↓ Navigate | Enter: Details"))?;
        stdout.queue(ResetColor)?;
        
        // Position cursor at input
        stdout.queue(MoveTo((2 + state.input_buffer.len()) as u16, footer_start + 1))?;
        
        stdout.flush()?;
        Ok(())
    }
    
    fn resize(&mut self, width: u16, height: u16) {
        self.terminal_size = (width, height);
    }
}

