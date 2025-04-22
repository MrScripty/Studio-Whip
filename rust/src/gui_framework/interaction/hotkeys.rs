use std::{collections::HashMap, fs, path::Path};
use thiserror::Error;
use winit::keyboard::{ModifiersState, PhysicalKey, KeyCode};
use bevy_reflect::Reflect;

// Error types for hotkey loading and parsing
#[derive(Error, Debug)]
pub enum HotkeyError {
    #[error("Hotkey configuration file not found at path: {0}")]
    FileNotFound(String),
    #[error("Failed to read hotkey configuration file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse hotkey configuration file (TOML): {0}")]
    ParseError(#[from] toml::de::Error),
}

// Holds the loaded hotkey mappings (Key Combo String -> Action String)
#[derive(Debug, Clone, Default, Reflect)]
pub struct HotkeyConfig {
    pub mappings: HashMap<String, String>,
}

impl HotkeyConfig {
    // Loads configuration from a TOML file at the given path
    pub fn load_config(path: &Path) -> Result<Self, HotkeyError> {
        if !path.exists() {
            return Ok(HotkeyConfig::default());
        }

        let content = fs::read_to_string(path)?;
        let mappings: HashMap<String, String> = toml::from_str(&content)?;

        Ok(HotkeyConfig { mappings })
    }

    // Retrieves the action string for a given key combination string, if mapped
    pub fn get_action(&self, key_combo: &str) -> Option<&String> {
        self.mappings.get(key_combo)
    }
}

// Formats a winit key event into a string like "Ctrl+Shift+A"
// Returns None for unhandled keys or modifier-only presses.
// NOTE: Requires accurate ModifiersState passed in.
// This function itself does not need reflection.
pub fn format_key_event(modifiers: ModifiersState, key: PhysicalKey) -> Option<String> {
    let mut parts = Vec::new();

    // Order: Ctrl, Alt, Shift, Super (for consistency)
    if modifiers.control_key() { parts.push("Ctrl"); }
    if modifiers.alt_key() { parts.push("Alt"); }
    if modifiers.shift_key() { parts.push("Shift"); }
    if modifiers.super_key() { parts.push("Super"); }

    let key_str = match key {
        PhysicalKey::Code(code) => match code {
            // Letters (A-Z)
            KeyCode::KeyA => "A", KeyCode::KeyB => "B", KeyCode::KeyC => "C", KeyCode::KeyD => "D",
            KeyCode::KeyE => "E", KeyCode::KeyF => "F", KeyCode::KeyG => "G", KeyCode::KeyH => "H",
            KeyCode::KeyI => "I", KeyCode::KeyJ => "J", KeyCode::KeyK => "K", KeyCode::KeyL => "L",
            KeyCode::KeyM => "M", KeyCode::KeyN => "N", KeyCode::KeyO => "O", KeyCode::KeyP => "P",
            KeyCode::KeyQ => "Q", KeyCode::KeyR => "R", KeyCode::KeyS => "S", KeyCode::KeyT => "T",
            KeyCode::KeyU => "U", KeyCode::KeyV => "V", KeyCode::KeyW => "W", KeyCode::KeyX => "X",
            KeyCode::KeyY => "Y", KeyCode::KeyZ => "Z",
            // Numbers (0-9)
            KeyCode::Digit0 => "0", KeyCode::Digit1 => "1", KeyCode::Digit2 => "2", KeyCode::Digit3 => "3",
            KeyCode::Digit4 => "4", KeyCode::Digit5 => "5", KeyCode::Digit6 => "6", KeyCode::Digit7 => "7",
            KeyCode::Digit8 => "8", KeyCode::Digit9 => "9",
            // Function Keys (F1-F12)
            KeyCode::F1 => "F1", KeyCode::F2 => "F2", KeyCode::F3 => "F3", KeyCode::F4 => "F4",
            KeyCode::F5 => "F5", KeyCode::F6 => "F6", KeyCode::F7 => "F7", KeyCode::F8 => "F8",
            KeyCode::F9 => "F9", KeyCode::F10 => "F10", KeyCode::F11 => "F11", KeyCode::F12 => "F12",
            // Common Special Keys
            KeyCode::Escape => "Escape",
            KeyCode::Space => "Space",
            KeyCode::Enter => "Enter",
            KeyCode::Backspace => "Backspace",
            KeyCode::Delete => "Delete",
            KeyCode::Tab => "Tab",
            KeyCode::ArrowUp => "ArrowUp", KeyCode::ArrowDown => "ArrowDown",
            KeyCode::ArrowLeft => "ArrowLeft", KeyCode::ArrowRight => "ArrowRight",
            // Add other KeyCodes as needed for hotkeys...
            _ => return None, // Ignore keys not explicitly handled here
        },
        PhysicalKey::Unidentified(_) => return None, // Ignore unidentified keys
    };

    parts.push(key_str);

    // Ensure it's not just modifiers being pressed
    if parts.len() > 1 || (parts.len() == 1 && !["Ctrl", "Alt", "Shift", "Super"].contains(&parts[0])) {
        Some(parts.join("+"))
    } else {
        None // Ignore modifier-only key events
    }
}