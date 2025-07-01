//! Terminal renderer trait and implementations

use crate::logging::{LogData, LogLevel};
use crossterm::{
    cursor::MoveTo,
    style::{self, Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
    ExecutableCommand, QueueableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Position},
    style::{Color as RatatuiColor, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io::{self, Write};

/// Frame state passed to the renderer
#[derive(Debug)]
pub struct CliFrameState<'a> {
    /// Logs to display (already filtered)
    pub logs: &'a [LogData],
    /// Current input buffer content
    pub input_buffer: &'a str,
    /// Current cursor position in input buffer (character-based)
    pub cursor_position: usize,
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
        let footer_lines = 4; // input box (3 lines) + help (1 line)
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
        
        // Draw input area with borders
        let footer_start = state.terminal_size.1.saturating_sub(footer_lines);
        let input_width = state.terminal_size.0 as usize;
        
        // Top border
        stdout.queue(MoveTo(0, footer_start))?;
        stdout.queue(SetForegroundColor(Color::Cyan))?;
        stdout.queue(Print("┌"))?;
        stdout.queue(Print("─".repeat(7)))?; // " Input "
        stdout.queue(Print(" Command Input "))?;
        stdout.queue(Print("─".repeat(input_width.saturating_sub(22))))?;
        stdout.queue(Print("┐"))?;
        stdout.queue(ResetColor)?;
        
        // Input line with side borders
        stdout.queue(MoveTo(0, footer_start + 1))?;
        stdout.queue(SetForegroundColor(Color::Cyan))?;
        stdout.queue(Print("│"))?;
        stdout.queue(ResetColor)?;
        stdout.queue(Print(" > "))?;
        stdout.queue(SetForegroundColor(Color::White))?;
        stdout.queue(Print(state.input_buffer))?;
        stdout.queue(ResetColor)?;
        
        // Clear rest of line and add right border
        let used_width = 4 + state.input_buffer.len(); // "│ > " + input text
        let remaining = input_width.saturating_sub(used_width + 1);
        stdout.queue(Print(" ".repeat(remaining)))?;
        stdout.queue(SetForegroundColor(Color::Cyan))?;
        stdout.queue(Print("│"))?;
        stdout.queue(ResetColor)?;
        
        // Bottom border
        stdout.queue(MoveTo(0, footer_start + 2))?;
        stdout.queue(SetForegroundColor(Color::Cyan))?;
        stdout.queue(Print("└"))?;
        stdout.queue(Print("─".repeat(input_width.saturating_sub(2))))?;
        stdout.queue(Print("┘"))?;
        stdout.queue(ResetColor)?;
        
        // Draw help line below the input box
        stdout.queue(MoveTo(0, footer_start + 3))?;
        stdout.queue(SetForegroundColor(Color::DarkGrey))?;
        stdout.queue(Print("Commands: /quit, /filter <level>, /clear, /save <path>, /copy | ↑↓ Navigate | Enter: Details"))?;
        stdout.queue(ResetColor)?;
        
        // Position cursor at the correct location in the input (inside the border)
        let cursor_x = 4 + state.cursor_position; // 4 for "│ > " prefix
        stdout.queue(MoveTo(cursor_x as u16, footer_start + 1))?;
        
        stdout.flush()?;
        Ok(())
    }
    
    fn resize(&mut self, width: u16, height: u16) {
        self.terminal_size = (width, height);
    }
}

/// Ratatui-based terminal renderer for better UI and event handling
pub struct RatatuiTerminalRenderer {
    /// Ratatui terminal instance
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    /// List state for log scrolling
    list_state: ListState,
}

impl RatatuiTerminalRenderer {
    /// Create a new ratatui terminal renderer
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        
        Ok(Self {
            terminal,
            list_state: ListState::default(),
        })
    }
    
    /// Get ratatui color for log level
    fn level_color(level: LogLevel) -> RatatuiColor {
        match level {
            LogLevel::Trace => RatatuiColor::Gray,
            LogLevel::Debug => RatatuiColor::Cyan,
            LogLevel::Info => RatatuiColor::Green,
            LogLevel::Warn => RatatuiColor::Yellow,
            LogLevel::Error => RatatuiColor::Red,
        }
    }
    
}

impl TerminalRenderer for RatatuiTerminalRenderer {
    fn draw(&mut self, state: CliFrameState) -> Result<(), Box<dyn std::error::Error>> {
        // Update list state for selection
        if let Some(selected) = state.selected_index {
            self.list_state.select(Some(selected.saturating_sub(state.scroll_offset)));
        } else {
            self.list_state.select(None);
        }
        
        // Pre-format log items to avoid borrowing issues
        let log_items: Vec<ListItem> = state.logs
            .iter()
            .skip(state.scroll_offset)
            .map(|log| Self::format_log_item_static(log))
            .collect();
        
        let input_text = format!("> {}", state.input_buffer);
        let status_message = state.status_message.to_string();
        let cursor_position = state.cursor_position;
        
        self.terminal.draw(|f| {
            let input_area = Self::draw_ui_static(f, &mut self.list_state, &log_items, &input_text, &status_message);
            
            // Set cursor position in the input area
            #[allow(clippy::cast_possible_truncation)]
            f.set_cursor_position(Position::new(
                // Add 1 for border, 2 for "> " prefix, then add cursor position
                input_area.x + 1 + cursor_position as u16 + 2,
                // Move one line down from the border to the input line
                input_area.y + 1,
            ));
        })?;
        
        Ok(())
    }
    
    fn resize(&mut self, _width: u16, _height: u16) {
        // Ratatui handles resize automatically
    }
}

impl RatatuiTerminalRenderer {
    /// Format a log entry as a ListItem (static version to avoid borrowing issues)
    fn format_log_item_static(log: &LogData) -> ListItem {
        let timestamp = log.metadata.timestamp
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Simple time formatting
        let hours = (timestamp / 3600) % 24;
        let minutes = (timestamp / 60) % 60;
        let seconds = timestamp % 60;
        let time_str = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
        
        let level_str = format!("{:5}", log.level.to_string());
        let target = &log.metadata.target;
        
        let dup_indicator = if log.duplicate_count > 0 {
            format!(" (x{})", log.duplicate_count + 1)
        } else {
            String::new()
        };
        
        let line = Line::from(vec![
            Span::styled(time_str, Style::default().fg(RatatuiColor::Gray)),
            Span::raw(" "),
            Span::styled(level_str, Style::default().fg(Self::level_color(log.level))),
            Span::raw(" "),
            Span::styled(target, Style::default().fg(RatatuiColor::Blue)),
            Span::raw(" "),
            Span::raw(&log.message),
            Span::styled(dup_indicator, Style::default().fg(RatatuiColor::DarkGray)),
        ]);
        
        ListItem::new(line)
    }
    
    /// Draw the UI using ratatui widgets (static version to avoid borrowing issues)
    /// Returns the input area Rect for cursor positioning
    fn draw_ui_static(
        frame: &mut Frame, 
        list_state: &mut ListState, 
        log_items: &[ListItem], 
        input_text: &str, 
        status_message: &str,
    ) -> ratatui::layout::Rect {
        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(1),    // Logs
                Constraint::Length(4), // Input box (3 lines) + Help (1 line)
            ])
            .split(frame.area());
        
        // Draw header
        let header = Paragraph::new("Whip UI Log Viewer")
            .style(Style::default().fg(RatatuiColor::Blue))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(header, chunks[0]);
        
        // Draw logs
        let logs_widget = List::new(log_items.to_vec())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(status_message)
            )
            .highlight_style(Style::default().bg(RatatuiColor::DarkGray))
            .highlight_symbol("► ");
        
        frame.render_stateful_widget(logs_widget, chunks[1], list_state);
        
        // Draw input area
        let help_text = "Commands: /quit, /filter <level>, /clear, /save <path>, /copy | ↑↓ Navigate | Enter: Details";
        
        let input_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Input area with borders
                Constraint::Length(1), // Help line
            ])
            .split(chunks[2]);
        
        let input_widget = Paragraph::new(input_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Command Input ")
                    .style(Style::default().fg(RatatuiColor::Cyan))
            )
            .style(Style::default().fg(RatatuiColor::White));
        frame.render_widget(input_widget, input_layout[0]);
        
        let help_widget = Paragraph::new(help_text)
            .style(Style::default().fg(RatatuiColor::Gray));
        frame.render_widget(help_widget, input_layout[1]);
        
        // Return the input area for cursor positioning
        input_layout[0]
    }
}

