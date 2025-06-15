use bevy_app::{App, AppExit, Plugin, Startup, Update};
use bevy_ecs::{prelude::*, schedule::SystemSet};
use bevy_transform::prelude::GlobalTransform;
use bevy_log::{info, error, warn};
use bevy_window::{PrimaryWindow, Window, WindowCloseRequested, CursorMoved};
use bevy_input::{
    keyboard::KeyCode, 
    ButtonInput,
    ButtonState,
    mouse::{MouseButton, MouseButtonInput},
};
use bevy_math::{Vec2, Rect, Affine3A};
use std::path::PathBuf;
use std::env;
// warning: unused import: `swash::Metrics as SwashMetrics` - REMOVED
use cosmic_text::Cursor;
use crate::gui_framework::components::{CursorState, TextSelection, CursorVisual};
use crate::{HotkeyResource, FontServerResource};
use crate::gui_framework::interaction::utils::get_cursor_at_position;
use crate::gui_framework::interaction::text_drag::text_drag_selection_system;
use crate::gui_framework::interaction::text_editing::text_editing_system;

// Import types from the crate root (lib.rs)
// (No specific types needed directly from lib.rs for this plugin)

// Import types/functions from the gui_framework
use crate::gui_framework::{
    interaction::hotkeys::{HotkeyConfig, HotkeyError},
    components::{Interaction, Visibility, Focus, EditableText, TextBufferCache},
    events::{EntityClicked, EntityDragged, HotkeyActionTriggered, YrsTextChanged, TextFocusChanged},
};

// Import resources used/managed by this plugin's systems
// HotkeyResource is defined in main.rs for now, but inserted by this plugin
use super::core::CoreSet;

// --- Local Resources ---

/// Enum describing the current high-level mouse interaction state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum MouseContextType {
    #[default]
    Idle, // Not interacting with anything specific
    DraggingShape, // Dragging a non-text entity
    TextInteraction, // Clicked down on editable text (potential drag selection start)
}

/// Resource holding the current mouse context.
#[derive(Resource, Default, Debug)]
pub(crate) struct MouseContext { // Make fields pub(crate) for access from text_drag
    pub(crate) context: MouseContextType,
}

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

        // --- Type Registration ---
        app.register_type::<Interaction>()
            .register_type::<HotkeyResource>() // Register the resource wrapper
            .register_type::<HotkeyConfig>()   // Register the inner config struct
            .register_type::<EntityClicked>()
            .register_type::<EntityDragged>()
            .register_type::<HotkeyActionTriggered>()
            .register_type::<EditableText>() 
            .register_type::<Focus>()
            .register_type::<TextSelection>();

        // --- Event Registration ---
        // Ensure events are registered if not already done elsewhere
        // (App::add_event is idempotent)
        app.add_event::<EntityClicked>()
            .add_event::<EntityDragged>()
            .add_event::<HotkeyActionTriggered>()
            .add_event::<YrsTextChanged>()
            .add_event::<TextFocusChanged>();
        app.init_resource::<MouseContext>();

        // --- System Setup ---
        app
            // Ensure hotkeys load after basic Vulkan setup is established by the core plugin
            .configure_sets(Startup, InteractionSet::LoadHotkeys.after(CoreSet::SetupVulkan))
            .add_systems(Startup, load_hotkeys_system.in_set(InteractionSet::LoadHotkeys))
            // Configure Update schedule order
            .configure_sets(Update, (
                // Ensure InputHandling runs before cursor management in CoreSet
                InteractionSet::InputHandling.before(CoreSet::ManageCursorVisual),
                // WindowClose can run later
                InteractionSet::WindowClose,
            ))
            .add_systems(Update,
                (
                    // interaction_system sets the context, text_drag_selection_system reads it
                    (interaction_system, hotkey_system, text_editing_system, text_drag_selection_system).chain().in_set(InteractionSet::InputHandling),
                    handle_close_request.in_set(InteractionSet::WindowClose),
                )
            );
    }
}

// --- Systems Moved/Created for this Plugin ---

/// Startup system: Loads hotkey configuration from file and inserts it as a resource.
fn load_hotkeys_system(mut commands: Commands) {
    let mut hotkey_path: Option<PathBuf> = None;
    let mut config_load_error: Option<String> = None;

    // Determine path relative to executable
    match env::current_exe() {
        Ok(mut exe_path) => {
            if exe_path.pop() { // Go up one level from executable file
                let path = exe_path.join("user").join("hotkeys.toml");
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


// Processes mouse input for clicks, drags, and text focus.
pub(crate) fn interaction_system(
    // Input resources
    mut yrs_text_changed_writer: EventWriter<YrsTextChanged>,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    windows: Query<&Window, With<PrimaryWindow>>,
    // Output events
    mut entity_clicked_writer: EventWriter<EntityClicked>,
    mut entity_dragged_writer: EventWriter<EntityDragged>,
    mut text_focus_writer: EventWriter<TextFocusChanged>,
    // Queries for entities
    interaction_query: Query<(Entity, &GlobalTransform, &Interaction, &Visibility), (Without<EditableText>, Without<CursorVisual>)>,
    editable_text_query: Query<(Entity, &GlobalTransform, &TextBufferCache, &Visibility), With<EditableText>>,
    focus_query: Query<Entity, With<Focus>>,
    // Resources
    mut mouse_context: ResMut<MouseContext>,
    // State for dragging
    mut drag_start_position: Local<Option<Vec2>>,
    mut dragged_entity: Local<Option<Entity>>,
    // Commands for adding/removing components
    mut commands: Commands,
    // Use ParamSet to handle conflicting queries on TextSelection
    mut selection_param_set: ParamSet<(
        Query<&mut TextSelection>, // p0: Mutable access
        Query<&TextSelection>,     // p1: Immutable access
    )>,
) {
    let Ok(primary_window) = windows.get_single() else { return; };
    let window_height = primary_window.height();
    let cursor_pos_window_opt = primary_window.cursor_position();

    // Helper enum for the unified hit-test result
    enum HitResult {
        Text { entity: Entity, z_depth: f32, cursor: Cursor },
        Shape { entity: Entity, z_depth: f32, interaction: Interaction },
    }

    // --- Process Mouse Button Events ---
    for event in mouse_button_input_events.read() {
        if event.button == MouseButton::Left {
            match event.state {
                ButtonState::Pressed => {
                    if let Some(cursor_pos_window) = cursor_pos_window_opt {
                        let cursor_pos_world = Vec2::new(cursor_pos_window.x, window_height - cursor_pos_window.y);
                        let mut top_hit: Option<HitResult> = None;

                        // --- 1. UNIFIED HIT-TESTING ---

                        // First, check for text hits
                        for (entity, transform, text_cache, visibility) in editable_text_query.iter() {
                            if !visibility.is_visible() { continue; }
                            if let Some(buffer) = text_cache.buffer.as_ref() {
                                if buffer.layout_runs().next().is_none() { continue; }

                                let mut bounds_min = Vec2::new(f32::MAX, f32::MAX);
                                let mut bounds_max = Vec2::new(f32::MIN, f32::MIN);
                                for run in buffer.layout_runs() {
                                    let line_start_x = run.glyphs.first().map_or(0.0, |g| g.x);
                                    bounds_min.x = bounds_min.x.min(line_start_x);
                                    bounds_max.x = bounds_max.x.max(line_start_x + run.line_w);
                                    bounds_min.y = bounds_min.y.min(run.line_top);
                                    bounds_max.y = bounds_max.y.max(run.line_y);
                                }
                                let mut local_bounds_ydown = Rect { min: bounds_min, max: bounds_max };
                                local_bounds_ydown.min -= Vec2::splat(2.0);
                                local_bounds_ydown.max += Vec2::splat(2.0);

                                let inverse_transform: Affine3A = transform.affine().inverse();
                                let cursor_pos_local_yup = inverse_transform.transform_point3(cursor_pos_world.extend(0.0)).truncate();
                                let cursor_pos_local_ydown = Vec2::new(cursor_pos_local_yup.x, -cursor_pos_local_yup.y);

                                if local_bounds_ydown.contains(cursor_pos_local_ydown) {
                                    if let Some(hit_cursor) = get_cursor_at_position(buffer, cursor_pos_local_ydown) {
                                        let z_depth = transform.translation().z;
                                        if top_hit.as_ref().map_or(true, |prev_hit| z_depth < match prev_hit {
                                            HitResult::Text { z_depth, .. } | HitResult::Shape { z_depth, .. } => *z_depth,
                                        }) {
                                            top_hit = Some(HitResult::Text { entity, z_depth, cursor: hit_cursor });
                                        }
                                    }
                                }
                            }
                        }

                        // Second, check for shape hits
                        for (entity, transform, interaction, visibility) in interaction_query.iter() {
                            if !visibility.is_visible() { continue; }
                            let inverse_transform: Affine3A = transform.affine().inverse();
                            let cursor_pos_local = inverse_transform.transform_point3(cursor_pos_world.extend(0.0)).truncate();
                            let bounds = Rect::from_center_half_size(Vec2::ZERO, Vec2::new(50.0, 50.0));

                            if bounds.contains(cursor_pos_local) {
                                let z_depth = transform.translation().z;
                                if top_hit.as_ref().map_or(true, |prev_hit| z_depth < match prev_hit {
                                    HitResult::Text { z_depth, .. } | HitResult::Shape { z_depth, .. } => *z_depth,
                                }) {
                                    top_hit = Some(HitResult::Shape { entity, z_depth, interaction: *interaction });
                                }
                            }
                        }

                        // --- 2. CENTRALIZED DECISION LOGIC ---

                        let previously_focused = focus_query.get_single().ok();

                        match top_hit {
                            // --- CASE: A TEXT ENTITY WAS CLICKED ---
                            Some(HitResult::Text { entity: target_entity, cursor, .. }) => {
                                mouse_context.context = MouseContextType::TextInteraction;
                                *dragged_entity = None;
                                *drag_start_position = None;

                                let mut global_byte_offset = 0;
                                let mut new_x_goal: Option<i32> = None;

                                if let Ok((_, _, text_cache, _)) = editable_text_query.get(target_entity) {
                                    if let Some(buffer) = text_cache.buffer.as_ref() {
                                        // Calculate global byte offset
                                        for i in 0..cursor.line {
                                            if let Some(line) = buffer.lines.get(i) {
                                                global_byte_offset += line.text().len();
                                                if i < buffer.lines.len() - 1 { global_byte_offset += 1; }
                                            }
                                        }
                                        global_byte_offset += cursor.index;

                                        // Manually find the x-position from the layout data
                                        'outer: for run in buffer.layout_runs() {
                                            if run.line_i == cursor.line {
                                                // Check if cursor is at the end of the line
                                                if cursor.index == run.glyphs.last().map_or(0, |g| g.end) {
                                                    // ** THE FIX IS HERE **
                                                    let line_start_x = run.glyphs.first().map_or(0.0, |g| g.x);
                                                    new_x_goal = Some((line_start_x + run.line_w) as i32);
                                                    break 'outer;
                                                }
                                                // Check if cursor is within one of the glyphs
                                                for glyph in run.glyphs.iter() {
                                                    if cursor.index >= glyph.start && cursor.index < glyph.end {
                                                        new_x_goal = Some(glyph.x as i32);
                                                        break 'outer;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                let new_cursor_pos = global_byte_offset;
                                let new_cursor_line = cursor.line;

                                if previously_focused != Some(target_entity) {
                                    if let Some(old_focus) = previously_focused {
                                        commands.entity(old_focus).remove::<(Focus, TextSelection, CursorState)>();
                                    }
                                    commands.entity(target_entity).insert((
                                        Focus,
                                        CursorState { position: new_cursor_pos, line: new_cursor_line, x_goal: new_x_goal },
                                        TextSelection { start: new_cursor_pos, end: new_cursor_pos },
                                    ));
                                    text_focus_writer.send(TextFocusChanged { entity: Some(target_entity) });
                                } else {
                                    commands.entity(target_entity).insert(CursorState { position: new_cursor_pos, line: new_cursor_line, x_goal: new_x_goal });
                                    if let Ok(mut selection) = selection_param_set.p0().get_mut(target_entity) {
                                        selection.start = new_cursor_pos;
                                        selection.end = new_cursor_pos;
                                    }
                                }
                            }

                            // --- CASE: A SHAPE ENTITY WAS CLICKED ---
                            Some(HitResult::Shape { entity: target_entity, interaction, .. }) => {
                                if let Some(old_focus) = previously_focused {
                                    commands.entity(old_focus).remove::<(Focus, TextSelection, CursorState)>();
                                    text_focus_writer.send(TextFocusChanged { entity: None });
                                }

                                if interaction.clickable {
                                    entity_clicked_writer.send(EntityClicked { entity: target_entity });
                                }
                                if interaction.draggable {
                                    mouse_context.context = MouseContextType::DraggingShape;
                                    *dragged_entity = Some(target_entity);
                                    *drag_start_position = Some(cursor_pos_world);
                                } else {
                                    mouse_context.context = MouseContextType::Idle;
                                    *dragged_entity = None;
                                    *drag_start_position = None;
                                }
                            }

                            // --- CASE: EMPTY SPACE WAS CLICKED ---
                            None => {
                                mouse_context.context = MouseContextType::Idle;
                                *dragged_entity = None;
                                *drag_start_position = None;

                                if let Some(old_focus) = previously_focused {
                                    commands.entity(old_focus).remove::<(Focus, TextSelection, CursorState)>();
                                    text_focus_writer.send(TextFocusChanged { entity: None });
                                }
                            }
                        }
                    }
                }
                ButtonState::Released => {
                    if mouse_context.context == MouseContextType::DraggingShape {
                        *drag_start_position = None;
                        *dragged_entity = None;
                    }
                    mouse_context.context = MouseContextType::Idle;
                }
            }
        }
    }

    // --- Process Mouse Motion Events ---
    if mouse_context.context == MouseContextType::DraggingShape {
        if let (Some(drag_entity), Some(start_pos)) = (*dragged_entity, *drag_start_position) {
            let mut last_cursor_pos = start_pos;

            if let Some(cursor_pos_window) = cursor_moved_events.read().last().map(|e| e.position) {
                last_cursor_pos = Vec2::new(cursor_pos_window.x, window_height - cursor_pos_window.y);
            }

            let delta = last_cursor_pos - start_pos;

            if delta.length_squared() > 0.0 {
                entity_dragged_writer.send(EntityDragged {
                    entity: drag_entity,
                    delta,
                });
                *drag_start_position = Some(last_cursor_pos);
            }
        }
    } else {
        cursor_moved_events.clear();
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

fn handle_close_request (
    mut ev_close: EventReader<WindowCloseRequested>,
    mut ev_app_exit: EventWriter<AppExit>,
) {
    // Check if any WindowCloseRequested event occurred this frame
    // Using read().next().is_some() is efficient as it stops after finding one event.
    if ev_close.read().next().is_some() {
        info!("WindowCloseRequested detected, sending AppExit (Interaction Plugin).");
        ev_app_exit.send(AppExit::Success);
    }
}