//! Command parsing for the CLI

use crate::logging::LogLevel;
use std::str::FromStr;

/// CLI commands
#[derive(Debug, Clone, PartialEq)]
pub enum CliCommand {
    /// Quit the CLI
    Quit,
    /// Filter logs by level or target
    Filter {
        level: Option<LogLevel>,
        target: Option<String>,
    },
    /// Clear the log store
    Clear,
    /// Save logs to a file
    Save {
        path: String,
    },
    /// Navigate up
    Up,
    /// Navigate down
    Down,
    /// Page up
    PageUp,
    /// Page down
    PageDown,
    /// Select/expand current log
    Select,
    /// Show help
    Help,
}

/// Parser for CLI commands
pub struct CommandParser;

impl CommandParser {
    /// Parse a command string
    pub fn parse(input: &str) -> Option<CliCommand> {
        let input = input.trim();
        
        // Handle slash commands
        if input.starts_with('/') {
            let parts: Vec<&str> = input[1..].split_whitespace().collect();
            if parts.is_empty() {
                return None;
            }
            
            match parts[0] {
                "q" | "quit" => Some(CliCommand::Quit),
                "c" | "clear" => Some(CliCommand::Clear),
                "h" | "help" => Some(CliCommand::Help),
                "f" | "filter" => {
                    if parts.len() > 1 {
                        // Try to parse as log level first
                        if let Ok(level) = LogLevel::from_str(parts[1]) {
                            Some(CliCommand::Filter {
                                level: Some(level),
                                target: None,
                            })
                        } else {
                            // Otherwise treat as target filter
                            Some(CliCommand::Filter {
                                level: None,
                                target: Some(parts[1].to_string()),
                            })
                        }
                    } else {
                        // Clear filter
                        Some(CliCommand::Filter {
                            level: None,
                            target: None,
                        })
                    }
                }
                "s" | "save" => {
                    if parts.len() > 1 {
                        Some(CliCommand::Save {
                            path: parts[1..].join(" "),
                        })
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else {
            None
        }
    }
}

impl FromStr for LogLevel {
    type Err = ();
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_quit_commands() {
        assert_eq!(CommandParser::parse("/q"), Some(CliCommand::Quit));
        assert_eq!(CommandParser::parse("/quit"), Some(CliCommand::Quit));
        assert_eq!(CommandParser::parse("  /quit  "), Some(CliCommand::Quit));
    }
    
    #[test]
    fn test_parse_filter_commands() {
        assert_eq!(
            CommandParser::parse("/filter debug"),
            Some(CliCommand::Filter {
                level: Some(LogLevel::Debug),
                target: None,
            })
        );
        
        assert_eq!(
            CommandParser::parse("/f error"),
            Some(CliCommand::Filter {
                level: Some(LogLevel::Error),
                target: None,
            })
        );
        
        assert_eq!(
            CommandParser::parse("/filter whip_ui"),
            Some(CliCommand::Filter {
                level: None,
                target: Some("whip_ui".to_string()),
            })
        );
    }
    
    #[test]
    fn test_parse_save_command() {
        assert_eq!(
            CommandParser::parse("/save logs.txt"),
            Some(CliCommand::Save {
                path: "logs.txt".to_string(),
            })
        );
        
        assert_eq!(
            CommandParser::parse("/s /tmp/my logs.txt"),
            Some(CliCommand::Save {
                path: "/tmp/my logs.txt".to_string(),
            })
        );
    }
    
    #[test]
    fn test_invalid_commands() {
        assert_eq!(CommandParser::parse(""), None);
        assert_eq!(CommandParser::parse("quit"), None);
        assert_eq!(CommandParser::parse("/unknown"), None);
        assert_eq!(CommandParser::parse("/save"), None); // Missing path
    }
}