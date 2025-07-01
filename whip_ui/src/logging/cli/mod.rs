//! Command-line interface for the logging service
//! 
//! This module provides a text-based UI for viewing and filtering logs
//! in real-time using crossterm for terminal manipulation.

pub mod renderer;
pub mod command;
pub mod session;
pub mod plugin;

pub use renderer::{TerminalRenderer, BasicTerminalRenderer, RatatuiTerminalRenderer, CliFrameState};
pub use command::{CliCommand, CommandParser};
pub use session::TerminalSession;
pub use plugin::{launch_cli, CliPlugin};

use std::error::Error;

/// Result type for CLI operations
pub type CliResult<T> = Result<T, Box<dyn Error + Send + Sync>>;