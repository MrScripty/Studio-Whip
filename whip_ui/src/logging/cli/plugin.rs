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
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
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
    /// Whether the display needs to be redrawn
    needs_redraw: bool,
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
            needs_redraw: true, // Initial draw needed
        }
    }
    
    /// Update cached logs from store
    fn update_logs(&mut self, log_store: &Arc<crate::logging::CentralLogStore>) {
        let new_logs = log_store.get_logs(self.filter.clone(), 0, usize::MAX);
        if new_logs.len() != self.cached_logs.len() {
            self.cached_logs = new_logs;
            self.needs_redraw = true;
        }
    }
    
    /// Handle keyboard input
    fn handle_key(&mut self, key: KeyEvent) -> Option<CliCommand> {
        match key.code {
            KeyCode::Enter => {
                if self.input_buffer.starts_with('/') {
                    let command = CommandParser::parse(&self.input_buffer);
                    self.input_buffer.clear();
                    self.needs_redraw = true;
                    return command;
                } else if self.selected_index.is_some() {
                    // Deselect
                    self.selected_index = None;
                    self.needs_redraw = true;
                } else {
                    // Select current item
                    if !self.cached_logs.is_empty() && self.scroll_offset < self.cached_logs.len() {
                        self.selected_index = Some(self.scroll_offset);
                        self.needs_redraw = true;
                    }
                }
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                    return Some(CliCommand::Quit);
                }
                self.input_buffer.push(c);
                self.needs_redraw = true;
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
                self.needs_redraw = true;
            }
            KeyCode::Up => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                    self.needs_redraw = true;
                }
            }
            KeyCode::Down => {
                if self.scroll_offset < self.cached_logs.len().saturating_sub(1) {
                    self.scroll_offset += 1;
                    self.needs_redraw = true;
                }
            }
            KeyCode::PageUp => {
                let page_size = self.terminal_size.1.saturating_sub(6) as usize;
                let new_offset = self.scroll_offset.saturating_sub(page_size);
                if new_offset != self.scroll_offset {
                    self.scroll_offset = new_offset;
                    self.needs_redraw = true;
                }
            }
            KeyCode::PageDown => {
                let page_size = self.terminal_size.1.saturating_sub(6) as usize;
                let new_offset = (self.scroll_offset + page_size)
                    .min(self.cached_logs.len().saturating_sub(1));
                if new_offset != self.scroll_offset {
                    self.scroll_offset = new_offset;
                    self.needs_redraw = true;
                }
            }
            KeyCode::Home => {
                if self.scroll_offset != 0 {
                    self.scroll_offset = 0;
                    self.needs_redraw = true;
                }
            }
            KeyCode::End => {
                let new_offset = self.cached_logs.len().saturating_sub(1);
                if self.scroll_offset != new_offset {
                    self.scroll_offset = new_offset;
                    self.needs_redraw = true;
                }
            }
            KeyCode::Esc => {
                let changed = self.selected_index.is_some() || !self.input_buffer.is_empty();
                self.selected_index = None;
                self.input_buffer.clear();
                if changed {
                    self.needs_redraw = true;
                }
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
                self.needs_redraw = true;
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
                self.needs_redraw = true;
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
                self.needs_redraw = true;
            }
            CliCommand::Help => {
                self.status_message = "Commands: /quit, /filter <level|target>, /clear, /save <path>".to_string();
                self.needs_redraw = true;
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
    println!("CLI: Starting event loop...");
    
    // Initialize terminal session
    let _session = TerminalSession::new()?;
    println!("CLI: Terminal session initialized");
    
    // Get log store reference
    let log_store = get_log_store()
        .ok_or("Log store not initialized")?;
    println!("CLI: Log store retrieved");
    
    // Initialize renderer and state
    let mut renderer = BasicTerminalRenderer::new();
    let mut state = CliState::new();
    println!("CLI: Renderer and state initialized");
    
    // Initial log fetch
    state.update_logs(&log_store);
    println!("CLI: Initial logs fetched, entering main loop...");
    
    // Main event loop
    loop {
        // Check for thread commands
        match receiver.try_recv() {
            Ok(CliThreadCommand::Shutdown) => {
                println!("CLI: Received shutdown command");
                break;
            }
            Err(TryRecvError::Disconnected) => {
                println!("CLI: Channel disconnected");
                break;
            }
            Err(TryRecvError::Empty) => {}
        }
        
        // Check for terminal resize
        let new_size = terminal::size().unwrap_or((80, 24));
        if new_size != state.terminal_size {
            state.terminal_size = new_size;
            renderer.resize(new_size.0, new_size.1);
            state.needs_redraw = true;
        }
        
        // Process keyboard events - use poll with timeout to avoid blocking
        // Only read if an event is available to prevent double processing
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    // Only process key press events to prevent double typing
                    if key.kind == KeyEventKind::Press {
                        if let Some(command) = state.handle_key(key) {
                            if state.process_command(command, &log_store) {
                                println!("CLI: Quit command received");
                                break; // Quit command
                            }
                        }
                    }
                }
                Event::Resize(width, height) => {
                    state.terminal_size = (width, height);
                    renderer.resize(width, height);
                }
                _ => {} // Ignore other events
            }
        }
        
        // Refresh logs periodically
        state.update_logs(&log_store);
        
        // Update status with log count
        if !state.status_message.starts_with("Commands:") && !state.status_message.starts_with("Saved") {
            let stats = log_store.get_stats();
            let new_status = format!(
                "Logs: {} (Total: {}, Dropped: {}) | Filter: {}",
                state.cached_logs.len(),
                stats.total_logs,
                stats.logs_dropped,
                if state.filter == LogFilter::default() { "None" } else { "Active" }
            );
            if new_status != state.status_message {
                state.status_message = new_status;
                state.needs_redraw = true;
            }
        }
        
        // Render frame only if needed
        if state.needs_redraw {
            let frame_state = CliFrameState {
                logs: &state.cached_logs,
                input_buffer: &state.input_buffer,
                status_message: &state.status_message,
                scroll_offset: state.scroll_offset,
                terminal_size: state.terminal_size,
                selected_index: state.selected_index,
            };
            
            renderer.draw(frame_state)?;
            state.needs_redraw = false;
        }
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
pub fn launch_cli() -> Result<thread::JoinHandle<()>, Box<dyn std::error::Error>> {
    let (sender, receiver) = bounded(10);
    
    let handle = thread::spawn(move || {
        // Keep sender alive by moving it into the thread
        let _sender = sender;
        if let Err(e) = cli_event_loop(receiver) {
            eprintln!("CLI error: {}", e);
        }
    });
    
    Ok(handle)
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