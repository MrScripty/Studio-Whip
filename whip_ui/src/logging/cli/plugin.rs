//! CLI plugin and main event loop

use crate::logging::{
    cli::{
        BasicTerminalRenderer, CliCommand, CliFrameState, CommandParser, TerminalRenderer,
        TerminalSession,
    },
    filter::{FilterConfig, LogFilter},
    get_log_store,
    LogData,
};
use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal,
};
use std::{
    fs::File,
    io::{self, Write},
    sync::Arc,
    thread,
    time::Duration,
};

/// Commands sent to the CLI thread
#[derive(Debug)]
pub enum CliThreadCommand {
    Shutdown,
}

/// State of the CLI application
struct CliState {
    /// Current input buffer
    input_buffer: String,
    /// Current status message
    status_message: String,
    /// Current scroll offset
    scroll_offset: usize,
    /// Selected log index for detail view
    selected_index: Option<usize>,
    /// Local filter for display
    filter: LogFilter,
    /// Cached logs
    cached_logs: Vec<LogData>,
    /// Terminal size
    terminal_size: (u16, u16),
}

impl CliState {
    fn new() -> Self {
        let terminal_size = terminal::size().unwrap_or((80, 24));
        Self {
            input_buffer: String::new(),
            status_message: "Ready. Type /help for commands.".to_string(),
            scroll_offset: 0,
            selected_index: None,
            filter: LogFilter::default(),
            cached_logs: Vec::new(),
            terminal_size,
        }
    }
    
    /// Update cached logs from store
    fn update_logs(&mut self, log_store: &Arc<crate::logging::CentralLogStore>) {
        self.cached_logs = log_store.get_logs(self.filter.clone(), 0, usize::MAX);
    }
    
    /// Handle keyboard input
    fn handle_key(&mut self, key: KeyEvent) -> Option<CliCommand> {
        match key.code {
            KeyCode::Enter => {
                if self.input_buffer.starts_with('/') {
                    let command = CommandParser::parse(&self.input_buffer);
                    self.input_buffer.clear();
                    return command;
                } else if self.selected_index.is_some() {
                    // Deselect
                    self.selected_index = None;
                } else {
                    // Select current item
                    if !self.cached_logs.is_empty() && self.scroll_offset < self.cached_logs.len() {
                        self.selected_index = Some(self.scroll_offset);
                    }
                }
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                    return Some(CliCommand::Quit);
                }
                self.input_buffer.push(c);
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Up => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
            }
            KeyCode::Down => {
                if self.scroll_offset < self.cached_logs.len().saturating_sub(1) {
                    self.scroll_offset += 1;
                }
            }
            KeyCode::PageUp => {
                let page_size = self.terminal_size.1.saturating_sub(6) as usize;
                self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
            }
            KeyCode::PageDown => {
                let page_size = self.terminal_size.1.saturating_sub(6) as usize;
                self.scroll_offset = (self.scroll_offset + page_size)
                    .min(self.cached_logs.len().saturating_sub(1));
            }
            KeyCode::Home => {
                self.scroll_offset = 0;
            }
            KeyCode::End => {
                self.scroll_offset = self.cached_logs.len().saturating_sub(1);
            }
            KeyCode::Esc => {
                self.selected_index = None;
                self.input_buffer.clear();
            }
            _ => {}
        }
        None
    }
    
    /// Process a command
    fn process_command(&mut self, command: CliCommand, log_store: &Arc<crate::logging::CentralLogStore>) -> bool {
        match command {
            CliCommand::Quit => return true,
            CliCommand::Clear => {
                log_store.clear();
                self.status_message = "Log store cleared.".to_string();
                self.cached_logs.clear();
                self.scroll_offset = 0;
                self.selected_index = None;
            }
            CliCommand::Filter { level, target } => {
                if level.is_none() && target.is_none() {
                    // Clear filter
                    self.filter = LogFilter::default();
                    self.status_message = "Filter cleared.".to_string();
                } else {
                    let mut config = FilterConfig::default();
                    if let Some(level) = level {
                        config.min_level = level;
                        self.filter = LogFilter::for_exact_level(level);
                        self.status_message = format!("Filtering by level: {}", level);
                    } else if let Some(target) = target {
                        config.target_filter = Some(target.clone());
                        self.filter = LogFilter::new(config);
                        self.status_message = format!("Filtering by target: {}", target);
                    }
                }
                self.update_logs(log_store);
                self.scroll_offset = 0;
                self.selected_index = None;
            }
            CliCommand::Save { path } => {
                match save_logs_to_file(&self.cached_logs, &path) {
                    Ok(count) => {
                        self.status_message = format!("Saved {} logs to {}", count, path);
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to save logs: {}", e);
                    }
                }
            }
            CliCommand::Help => {
                self.status_message = "Commands: /quit, /filter <level|target>, /clear, /save <path>".to_string();
            }
            _ => {}
        }
        false
    }
}

/// Save logs to a file
fn save_logs_to_file(logs: &[LogData], path: &str) -> io::Result<usize> {
    let mut file = File::create(path)?;
    for log in logs {
        writeln!(file, "{}", log.format_display())?;
    }
    Ok(logs.len())
}

/// Main CLI event loop
fn cli_event_loop(receiver: Receiver<CliThreadCommand>) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize terminal session
    let _session = TerminalSession::new()?;
    
    // Get log store reference
    let log_store = get_log_store()
        .ok_or("Log store not initialized")?;
    
    // Initialize renderer and state
    let mut renderer = BasicTerminalRenderer::new();
    let mut state = CliState::new();
    
    // Initial log fetch
    state.update_logs(&log_store);
    
    // Main event loop
    loop {
        // Check for thread commands
        match receiver.try_recv() {
            Ok(CliThreadCommand::Shutdown) => break,
            Err(TryRecvError::Disconnected) => break,
            Err(TryRecvError::Empty) => {}
        }
        
        // Check for terminal resize
        let new_size = terminal::size().unwrap_or((80, 24));
        if new_size != state.terminal_size {
            state.terminal_size = new_size;
            renderer.resize(new_size.0, new_size.1);
        }
        
        // Process keyboard events
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if let Some(command) = state.handle_key(key) {
                    if state.process_command(command, &log_store) {
                        break; // Quit command
                    }
                }
            }
        }
        
        // Refresh logs periodically
        state.update_logs(&log_store);
        
        // Update status with log count
        if !state.status_message.starts_with("Commands:") && !state.status_message.starts_with("Saved") {
            let stats = log_store.get_stats();
            state.status_message = format!(
                "Logs: {} (Total: {}, Dropped: {}) | Filter: {}",
                state.cached_logs.len(),
                stats.total_logs,
                stats.logs_dropped,
                if state.filter == LogFilter::default() { "None" } else { "Active" }
            );
        }
        
        // Render frame
        let frame_state = CliFrameState {
            logs: &state.cached_logs,
            input_buffer: &state.input_buffer,
            status_message: &state.status_message,
            scroll_offset: state.scroll_offset,
            terminal_size: state.terminal_size,
            selected_index: state.selected_index,
        };
        
        renderer.draw(frame_state)?;
    }
    
    Ok(())
}

/// CLI plugin for Bevy
pub struct CliPlugin {
    /// Channel sender for controlling the CLI thread
    pub sender: Option<Sender<CliThreadCommand>>,
}

impl Default for CliPlugin {
    fn default() -> Self {
        Self { sender: None }
    }
}

impl bevy_app::Plugin for CliPlugin {
    fn build(&self, _app: &mut bevy_app::App) {
        // The plugin doesn't add systems, just provides the launch capability
    }
}

/// Launch the CLI in a separate thread
pub fn launch_cli() -> Result<Sender<CliThreadCommand>, Box<dyn std::error::Error>> {
    let (sender, receiver) = bounded(10);
    
    thread::spawn(move || {
        if let Err(e) = cli_event_loop(receiver) {
            eprintln!("CLI error: {}", e);
        }
    });
    
    Ok(sender)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cli_state_creation() {
        let state = CliState::new();
        assert!(state.input_buffer.is_empty());
        assert_eq!(state.scroll_offset, 0);
        assert!(state.selected_index.is_none());
    }
}