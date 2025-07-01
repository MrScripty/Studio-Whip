//! Terminal session management with automatic cleanup

use crossterm::{
    cursor,
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use std::io::{self, stdout};
use std::panic;

/// RAII guard for terminal session
/// Ensures terminal is restored to usable state on drop
pub struct TerminalSession {
    /// Whether we successfully initialized
    initialized: bool,
}

impl TerminalSession {
    /// Create a new terminal session
    pub fn new() -> io::Result<Self> {
        // Set up panic hook to restore terminal
        let original_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            // Restore terminal before panicking
            let _ = Self::cleanup();
            original_hook(panic_info);
        }));
        
        // Enter raw mode and alternate screen
        terminal::enable_raw_mode()?;
        stdout()
            .execute(EnterAlternateScreen)?
            .execute(EnableMouseCapture)?
            .execute(cursor::Hide)?;
        
        Ok(Self {
            initialized: true,
        })
    }
    
    /// Clean up terminal state
    fn cleanup() -> io::Result<()> {
        // Restore terminal
        let _ = stdout()
            .execute(cursor::Show)?
            .execute(DisableMouseCapture)?
            .execute(LeaveAlternateScreen)?;
        terminal::disable_raw_mode()
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if self.initialized {
            // Best effort cleanup
            let _ = Self::cleanup();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_session_creation() {
        // Can't really test terminal operations in unit tests
        // Just ensure the types compile correctly
        let _session_type: Option<TerminalSession> = None;
    }
}