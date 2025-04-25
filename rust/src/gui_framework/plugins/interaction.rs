use bevy_app::{App, AppExit, Plugin, Startup, Update};
use bevy_ecs::{prelude::*, schedule::SystemSet};
use bevy_log::{info, error, warn};
use bevy_window::{PrimaryWindow, Window, WindowCloseRequested, CursorMoved};
use bevy_input::{keyboard::KeyCode, mouse::MouseButton, ButtonInput};
use bevy_math::Vec2;
use std::path::PathBuf;
use std::env;

// Import types from the crate root (lib.rs)
// (No specific types needed directly from lib.rs for this plugin)

// Import types/functions from the gui_framework
use crate::gui_framework::{
    interaction::hotkeys::{HotkeyConfig, HotkeyError},
    components::{Interaction, ShapeData, Visibility}, // Need ShapeData/Visibility for interaction_system query
    events::{EntityClicked, EntityDragged, HotkeyActionTriggered},
};

// Import resources used/managed by this plugin's systems
// HotkeyResource is defined in main.rs for now, but inserted by this plugin
use crate::HotkeyResource; // Assuming HotkeyResource is defined in main.rs or lib.rs

// --- System Sets ---
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum InteractionSet {
    LoadHotkeys,
    InputHandling, // Group mouse/keyboard input processing
    WindowClose,
}

// --- Interaction Plugin Definition ---

pub struct GuiFrameworkInteractionPlugin;

impl Plugin for GuiFrameworkInteractionPlugin {
    fn build(&self, app: &mut App) {
        info!("Building GuiFrameworkInteractionPlugin...");

        // --- Type Registration ---
        app.register_type::<Interaction>()
           .register_type::<HotkeyResource>() // Register the resource wrapper
           .register_type::<HotkeyConfig>()   // Register the inner config struct
           .register_type::<EntityClicked>()
           .register_type::<EntityDragged>()
           .register_type::<HotkeyActionTriggered>();

        // --- Event Registration ---
        // Ensure events are registered if not already done elsewhere
        // (App::add_event is idempotent)
        app.add_event::<EntityClicked>()
           .add_event::<EntityDragged>()
           .add_event::<HotkeyActionTriggered>();

        // --- System Setup ---
        app
            .add_systems(Startup, load_hotkeys_system.in_set(InteractionSet::LoadHotkeys))
            .add_systems(Update,
                (
                    (interaction_system, hotkey_system).in_set(InteractionSet::InputHandling),
                    handle_close_request.in_set(InteractionSet::WindowClose),
                )
            );

        info!("GuiFrameworkInteractionPlugin built.");
    }
}

// --- Systems Moved/Created for this Plugin ---

/// Startup system: Loads hotkey configuration from file and inserts it as a resource.
fn load_hotkeys_system(mut commands: Commands) {
    info!("Running load_hotkeys_system (Interaction Plugin)...");
    let mut hotkey_path: Option<PathBuf> = None;
    let mut config_load_error: Option<String> = None;

    // Determine path relative to executable
    match env::current_exe() {
        Ok(mut exe_path) => {
            if exe_path.pop() { // Go up one level from executable file
                let path = exe_path.join("user").join("hotkeys.toml");
                info!("[HotkeyLoader] Looking for hotkeys file at: {:?}", path);
                hotkey_path = Some(path);
            } else {
                config_load_error = Some("Could not get executable directory.".to_string());
            }
        }
        Err(e) => {
            config_load_error = Some(format!("Failed to get current executable path: {}", e));
        }
    }

    // Load config or use default
    let config = match hotkey_path {
        Some(path) => {
            if path.exists() {
                HotkeyConfig::load_config(&path).unwrap_or_else(|e| {
                    match e {
                        HotkeyError::ReadError(io_err) => error!("[HotkeyLoader] Error reading hotkey file '{:?}': {}", path, io_err),
                        HotkeyError::ParseError(toml_err) => error!("[HotkeyLoader] Error parsing hotkey file '{:?}': {}", path, toml_err),
                        HotkeyError::FileNotFound(_) => error!("[HotkeyLoader] Hotkey file disappeared between check and load: {:?}", path), // Should not happen
                    }
                    warn!("[HotkeyLoader] Using default empty hotkey configuration due to error.");
                    HotkeyConfig::default()
                })
            } else {
                warn!("[HotkeyLoader] Hotkey file '{:?}' not found. Using default empty configuration.", path);
                HotkeyConfig::default()
            }
        }
        None => {
            error!("[HotkeyLoader] {}", config_load_error.unwrap_or("Hotkey path could not be determined.".to_string()));
            warn!("[HotkeyLoader] Using default empty hotkey configuration.");
            HotkeyConfig::default()
        }
    };

    // Insert the loaded (or default) config as a resource
    commands.insert_resource(HotkeyResource(config));
    info!("Hotkey configuration loaded and inserted as resource (Interaction Plugin).");
}


/// Update system: Processes mouse input for clicking and dragging.
fn interaction_system(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut cursor_evr: EventReader<CursorMoved>,
    mut entity_clicked_evw: EventWriter<EntityClicked>,
    mut entity_dragged_evw: EventWriter<EntityDragged>,
    // Query needs Transform, ShapeData, Visibility, Interaction
    query: Query<(Entity, &bevy_transform::prelude::Transform, &ShapeData, &Visibility, &Interaction)>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    mut drag_state: Local<Option<(Entity, Vec2)>>, // Local state for tracking drag
) {
    let Ok(window) = window_q.get_single() else { return };
    let window_height = window.height();

    let mut current_cursor_pos: Option<Vec2> = None;
    for ev in cursor_evr.read() {
        current_cursor_pos = Some(ev.position);
    }

    // --- Dragging Logic ---
    if mouse_button_input.pressed(MouseButton::Left) {
        if let Some(cursor_pos) = current_cursor_pos {
            // If already dragging an entity
            if let Some((dragged_entity, last_pos)) = *drag_state {
                let delta = cursor_pos - last_pos;
                // Send drag event only if mouse moved significantly
                if delta.length_squared() > 0.0 {
                    entity_dragged_evw.send(EntityDragged { entity: dragged_entity, delta });
                    // Update last position for next frame's delta calculation
                    *drag_state = Some((dragged_entity, cursor_pos));
                }
            }
            // If not currently dragging, check if starting a drag
            else {
                let mut top_hit: Option<(Entity, f32)> = None;
                // Iterate through draggable entities
                for (entity, transform, shape, visibility, interaction) in query.iter() {
                    if !visibility.is_visible() || !interaction.draggable { continue; }

                    let pos = transform.translation;
                    // Simple bounding box check using vertex data
                    let (min_x, max_x, min_y, max_y) = shape.vertices.iter().fold(
                        (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY),
                        |acc, v| (acc.0.min(v.position[0]), acc.1.max(v.position[0]), acc.2.min(v.position[1]), acc.3.max(v.position[1]))
                    );
                    let world_min_x = min_x + pos.x; let world_max_x = max_x + pos.x;
                    let world_min_y = min_y + pos.y; let world_max_y = max_y + pos.y;

                    // Adjust cursor Y for Bevy's coordinate system (Y-down from top-left) vs world (Y-up from bottom-left)
                    let adjusted_cursor_y = window_height - cursor_pos.y;

                    // Check if cursor is within the bounding box
                    if cursor_pos.x >= world_min_x && cursor_pos.x <= world_max_x &&
                       adjusted_cursor_y >= world_min_y && adjusted_cursor_y <= world_max_y {
                        // Prioritize entity with higher Z-index (closer to camera)
                        if top_hit.is_none() || pos.z > top_hit.unwrap().1 {
                            top_hit = Some((entity, pos.z));
                        }
                    }
                }
                // If a draggable entity was hit, start dragging it
                if let Some((hit_entity, _)) = top_hit {
                    *drag_state = Some((hit_entity, cursor_pos));
                }
            }
        }
    }

    // --- Stop Dragging ---
    if mouse_button_input.just_released(MouseButton::Left) {
        *drag_state = None; // Clear drag state when mouse button is released
    }

    // --- Clicking Logic ---
    // Check only if the mouse button was just pressed (not held) and not currently dragging
    if mouse_button_input.just_pressed(MouseButton::Left) && drag_state.is_none() {
        if let Some(cursor_pos) = current_cursor_pos {
            let mut top_hit: Option<(Entity, f32)> = None;
            // Iterate through clickable entities
            for (entity, transform, shape, visibility, interaction) in query.iter() {
                 if !visibility.is_visible() || !interaction.clickable { continue; }

                let pos = transform.translation;
                // Simple bounding box check
                let (min_x, max_x, min_y, max_y) = shape.vertices.iter().fold(
                    (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY),
                    |acc, v| (acc.0.min(v.position[0]), acc.1.max(v.position[0]), acc.2.min(v.position[1]), acc.3.max(v.position[1]))
                );
                let world_min_x = min_x + pos.x; let world_max_x = max_x + pos.x;
                let world_min_y = min_y + pos.y; let world_max_y = max_y + pos.y;
                let adjusted_cursor_y = window_height - cursor_pos.y;

                if cursor_pos.x >= world_min_x && cursor_pos.x <= world_max_x &&
                   adjusted_cursor_y >= world_min_y && adjusted_cursor_y <= world_max_y {
                    if top_hit.is_none() || pos.z > top_hit.unwrap().1 {
                        top_hit = Some((entity, pos.z));
                    }
                }
            }
            // If a clickable entity was hit, send the click event
            if let Some((hit_entity, _)) = top_hit {
                entity_clicked_evw.send(EntityClicked { entity: hit_entity });
                // info!("Sent EntityClicked event for {:?}", hit_entity); // Optional log
            }
        }
    }
}


/// Update system: Detects keyboard input and sends HotkeyActionTriggered events.
fn hotkey_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    hotkey_config: Res<HotkeyResource>, // Access the resource loaded at startup
    mut hotkey_evw: EventWriter<HotkeyActionTriggered>,
) {
    let ctrl = keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let alt = keyboard_input.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);
    let shift = keyboard_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let super_key = keyboard_input.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]);

    // Iterate through keys just pressed this frame
    for keycode in keyboard_input.get_just_pressed() {
        let mut parts = Vec::new();
        // Add modifiers first
        if ctrl { parts.push("Ctrl"); }
        if alt { parts.push("Alt"); }
        if shift { parts.push("Shift"); }
        if super_key { parts.push("Super"); }

        // Format the key itself
        let key_str = match keycode {
            KeyCode::KeyA => "A", KeyCode::KeyB => "B", KeyCode::KeyC => "C", KeyCode::KeyD => "D", KeyCode::KeyE => "E",
            KeyCode::KeyF => "F", KeyCode::KeyG => "G", KeyCode::KeyH => "H", KeyCode::KeyI => "I", KeyCode::KeyJ => "J",
            KeyCode::KeyK => "K", KeyCode::KeyL => "L", KeyCode::KeyM => "M", KeyCode::KeyN => "N", KeyCode::KeyO => "O",
            KeyCode::KeyP => "P", KeyCode::KeyQ => "Q", KeyCode::KeyR => "R", KeyCode::KeyS => "S", KeyCode::KeyT => "T",
            KeyCode::KeyU => "U", KeyCode::KeyV => "V", KeyCode::KeyW => "W", KeyCode::KeyX => "X", KeyCode::KeyY => "Y",
            KeyCode::KeyZ => "Z",
            KeyCode::Digit0 => "0", KeyCode::Digit1 => "1", KeyCode::Digit2 => "2", KeyCode::Digit3 => "3", KeyCode::Digit4 => "4",
            KeyCode::Digit5 => "5", KeyCode::Digit6 => "6", KeyCode::Digit7 => "7", KeyCode::Digit8 => "8", KeyCode::Digit9 => "9",
            KeyCode::F1 => "F1", KeyCode::F2 => "F2", KeyCode::F3 => "F3", KeyCode::F4 => "F4", KeyCode::F5 => "F5",
            KeyCode::F6 => "F6", KeyCode::F7 => "F7", KeyCode::F8 => "F8", KeyCode::F9 => "F9", KeyCode::F10 => "F10",
            KeyCode::F11 => "F11", KeyCode::F12 => "F12",
            KeyCode::Escape => "Escape", KeyCode::Space => "Space", KeyCode::Enter => "Enter", KeyCode::Backspace => "Backspace",
            KeyCode::Delete => "Delete", KeyCode::Tab => "Tab",
            KeyCode::ArrowUp => "ArrowUp", KeyCode::ArrowDown => "ArrowDown", KeyCode::ArrowLeft => "ArrowLeft", KeyCode::ArrowRight => "ArrowRight",
            // Add other keys as needed
            _ => continue, // Skip unhandled keys
        };
        parts.push(key_str);
        let key_combo_str = parts.join("+"); // e.g., "Ctrl+Shift+S"

        // Check if this combo maps to an action in the config
        if let Some(action) = hotkey_config.0.get_action(&key_combo_str) {
            // info!("Hotkey detected: {} -> Action: {}", key_combo_str, action); // Optional log
            hotkey_evw.send(HotkeyActionTriggered { action: action.clone() });
        }
    }
}

/// Update system: Handles WindowCloseRequested events (e.g., clicking the 'X' button) -> AppExit.
fn handle_close_request(
    mut ev_close: EventReader<WindowCloseRequested>,
    mut ev_app_exit: EventWriter<AppExit>,
) {
    // Check if any WindowCloseRequested event occurred this frame
    if ev_close.read().next().is_some() {
        info!("WindowCloseRequested detected, sending AppExit (Interaction Plugin).");
        ev_app_exit.send(AppExit::Success); // Send the AppExit event
    }
}