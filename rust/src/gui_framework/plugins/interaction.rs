use bevy_app::{App, AppExit, Plugin, Startup, Update};
use bevy_ecs::{prelude::*, schedule::SystemSet};
use bevy_transform::prelude::GlobalTransform;
use bevy_log::{info, error, warn};
use bevy_window::{PrimaryWindow, Window, WindowCloseRequested, CursorMoved};
use bevy_input::{
    keyboard::KeyCode, 
    ButtonInput,
    ButtonState,
    mouse::{MouseButton, MouseButtonInput, MouseMotion},
};
use bevy_math::{Vec2, Rect, Affine3A};
use std::path::PathBuf;
use std::env;
use std::sync::{Arc, Mutex};
use swash::Metrics as SwashMetrics;
use cosmic_text::Cursor;
use crate::gui_framework::components::{CursorState, TextSelection};
use crate::{HotkeyResource, FontServerResource};
use crate::gui_framework::interaction::utils::get_cursor_at_position;

// Import types from the crate root (lib.rs)
// (No specific types needed directly from lib.rs for this plugin)

// Import types/functions from the gui_framework
use crate::gui_framework::{
    interaction::hotkeys::{HotkeyConfig, HotkeyError},
    components::{Interaction, ShapeData, Visibility, Focus, EditableText, TextLayoutOutput, TextBufferCache},
    events::{EntityClicked, EntityDragged, HotkeyActionTriggered, YrsTextChanged, TextFocusChanged},
};

// Import resources used/managed by this plugin's systems
// HotkeyResource is defined in main.rs for now, but inserted by this plugin
use super::core::CoreSet;

// --- Local Resources ---

/// Enum describing the current high-level mouse interaction state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum MouseContextType {
    #[default]
    Idle, // Not interacting with anything specific
    DraggingShape, // Dragging a non-text entity
    TextInteraction, // Clicked down on editable text (potential drag selection start)
}

/// Resource holding the current mouse context.
#[derive(Resource, Default)]
struct MouseContext {
    context: MouseContextType,
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
                    (interaction_system, hotkey_system).in_set(InteractionSet::InputHandling),
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


// Update system: Processes mouse input for clicks, drags, and text focus.
pub fn interaction_system(
    // Input resources
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    windows: Query<&Window, With<PrimaryWindow>>,
    // Output events
    mut entity_clicked_writer: EventWriter<EntityClicked>,
    mut entity_dragged_writer: EventWriter<EntityDragged>,
    mut text_focus_writer: EventWriter<TextFocusChanged>,
    // Queries for entities
    interaction_query: Query<(Entity, &GlobalTransform, &Interaction, &Visibility), Without<EditableText>>, // Exclude EditableText
    editable_text_query: Query<(Entity, &GlobalTransform, &TextBufferCache, &Visibility), With<EditableText>>,
    focus_query: Query<Entity, With<Focus>>, // Query for entities that currently HAVE focus
    // Resources
    font_server_res: Res<FontServerResource>,
    mut mouse_context: ResMut<MouseContext>, // Add MouseContext resource
    // State for dragging
    mut drag_start_position: Local<Option<Vec2>>,
    mut dragged_entity: Local<Option<Entity>>,
    // Commands for adding/removing components
    mut commands: Commands,
    // Query for modifying TextSelection
    mut text_selection_query: Query<&mut TextSelection>,
) {
    let Ok(primary_window) = windows.get_single() else { return; };
    let window_height = primary_window.height();

    // --- Process Mouse Button Events ---
    for event in mouse_button_input_events.read() {
        if event.button == MouseButton::Left {
            match event.state {
                ButtonState::Pressed => {
                    // --- Handle Click Down (Potential Drag Start or Focus Change) ---
                    if let Some(cursor_pos_window) = primary_window.cursor_position() {
                        let cursor_pos_world = Vec2::new(cursor_pos_window.x, window_height - cursor_pos_window.y);

                        // Reset context at the start of a press
                        mouse_context.context = MouseContextType::Idle;

                        let mut clicked_on_something = false;
                        let mut clicked_on_text_entity: Option<Entity> = None;
                        let mut text_hit_details: Option<(Entity, Cursor)> = None;

                        // --- 1. Check Editable Text Hit Detection (Overall BBox + buffer.hit()) ---
                        let Ok(mut font_server_guard) = font_server_res.0.lock() else {
                            error!("Failed to lock FontServer in interaction_system");
                            continue;
                        };

                        for (entity, transform, text_cache, visibility) in editable_text_query.iter() {
                            if !visibility.is_visible() { continue; }
                            let Some(buffer) = text_cache.buffer.as_ref() else { continue; };

                            // Calculate overall bounding box in cosmic-text local coords (Y-down)
                            let mut overall_min_x = f32::MAX;
                            let mut overall_max_x = f32::MIN;
                            let mut overall_min_y = f32::MAX; // Top Y
                            let mut overall_max_y = f32::MIN; // Bottom Y

                            for run in buffer.layout_runs() {
                                // Use line_top and calculated line_bottom even if glyphs are empty or width is zero
                                // This ensures empty lines contribute to vertical bounds
                                let run_min_x = if run.glyphs.is_empty() { 0.0 } else { run.glyphs[0].x }; // Default X if no glyphs
                                let run_max_x = run_min_x + run.line_w;
                                let run_min_y = run.line_top; // Top edge

                                // Calculate max descent for this line (even if empty, descent is 0)
                                let mut max_scaled_descent = 0.0f32;
                                for glyph in run.glyphs { // This loop won't run if glyphs is empty
                                    if let Some(font) = font_server_guard.font_system.get_font(glyph.font_id) {
                                        let swash_font = font.as_swash();
                                        let metrics: SwashMetrics = swash_font.metrics(&[]);
                                        if metrics.units_per_em > 0 {
                                            let scale = glyph.font_size / metrics.units_per_em as f32;
                                            max_scaled_descent = max_scaled_descent.max(metrics.descent.abs() * scale);
                                        }
                                    }
                                }
                                let run_max_y = run.line_y + max_scaled_descent; // Bottom edge

                                overall_min_x = overall_min_x.min(run_min_x);
                                overall_max_x = overall_max_x.max(run_max_x);
                                overall_min_y = overall_min_y.min(run_min_y);
                                overall_max_y = overall_max_y.max(run_max_y);
                            }

                            // Check if bounds were actually updated (i.e., text wasn't empty / had no runs)
                            if overall_min_x <= overall_max_x && overall_min_y <= overall_max_y {
                                // Create overall Rect in cosmic-text coords (Y-down)
                                let mut overall_rect_local_ydown = Rect::from_corners(
                                    Vec2::new(overall_min_x, overall_min_y), // Top-Left
                                    Vec2::new(overall_max_x, overall_max_y)  // Bottom-Right
                                );

                                // Add padding
                                overall_rect_local_ydown.min -= Vec2::splat(2.0);
                                overall_rect_local_ydown.max += Vec2::splat(2.0);

                                // Flip Y axis for Bevy's local coords (Y-up)
                                let overall_rect_local_yup = Rect::from_corners(
                                    Vec2::new(overall_rect_local_ydown.min.x, -overall_rect_local_ydown.max.y),
                                    Vec2::new(overall_rect_local_ydown.max.x, -overall_rect_local_ydown.min.y)
                                );

                                // Transform cursor world position to entity's local space (Y-up)
                                let inverse_transform: Affine3A = transform.affine().inverse();
                                let cursor_pos_local_yup = inverse_transform.transform_point3(cursor_pos_world.extend(0.0)).truncate();

                                // Perform hit check using the Y-up Rect
                                if overall_rect_local_yup.contains(cursor_pos_local_yup) {
                                    // If overall box hit, use utility function with Y-down local coords
                                    let cursor_pos_local_ydown = Vec2::new(cursor_pos_local_yup.x, -cursor_pos_local_yup.y);
                                    // Use the utility function here
                                    if let Some(hit_cursor) = get_cursor_at_position(buffer, cursor_pos_local_ydown) {
                                        info!("Hit text entity {:?} at cursor: {:?}", entity, hit_cursor);
                                        clicked_on_text_entity = Some(entity);
                                        text_hit_details = Some((entity, hit_cursor)); // Store hit_cursor
                                        // Set context now that we know we hit text
                                        mouse_context.context = MouseContextType::TextInteraction;
                                        clicked_on_something = true;
                                        break; // Found hit on this entity
                                    }
                                }
                            }
                            if clicked_on_something { break; } // Stop checking other text entities
                        }
                        drop(font_server_guard);

                        // --- 2. Check Non-Text Interactable Hit Detection (If no text hit) ---
                        if !clicked_on_something {
                            let mut top_entity: Option<(Entity, f32, bool, bool)> = None;

                            for (entity, transform, interaction, visibility) in interaction_query.iter() {
                                if !visibility.is_visible() { continue; }

                                let inverse_transform: Affine3A = transform.affine().inverse();
                                let cursor_pos_local = inverse_transform.transform_point3(cursor_pos_world.extend(0.0)).truncate();

                                let half_size = Vec2::new(50.0, 50.0); // Placeholder
                                let bounds = Rect::from_center_half_size(Vec2::ZERO, half_size);

                                if bounds.contains(cursor_pos_local) {
                                    let z_depth = transform.translation().z;
                                    if top_entity.is_none() || z_depth < top_entity.unwrap().1 {
                                        top_entity = Some((entity, z_depth, interaction.clickable, interaction.draggable));
                                    }
                                }
                            }

                            if let Some((entity, _z, clickable, draggable)) = top_entity {
                                clicked_on_something = true;
                                if clickable {
                                    entity_clicked_writer.send(EntityClicked { entity });
                                }
                                if draggable {
                                    *drag_start_position = Some(cursor_pos_world);
                                    *dragged_entity = Some(entity);
                                    mouse_context.context = MouseContextType::DraggingShape;
                                }
                            }
                        }


                        // --- 3. Handle Focus Change using Focus Component ---
                        let mut focus_event_to_send: Option<Option<Entity>> = None; // Track if event needs sending
                        let mut entity_to_unfocus: Option<Entity> = None;

                        // Determine which entity currently has focus, if any
                        for current_focus_entity in focus_query.iter() {
                            entity_to_unfocus = Some(current_focus_entity);
                            break; // Should only be one
                        }

                        // Update CursorState and TextSelection based on text_hit_details
                        if let Some((target_text_entity, hit_cursor)) = text_hit_details {
                            let new_cursor_pos = hit_cursor.index; // Get byte offset
                            let new_cursor_line = hit_cursor.line; // Get line index

                            // --- Handle Focus Change ---
                            if entity_to_unfocus != Some(target_text_entity) {
                                // Focus is changing TO this text entity
                                if let Some(old_focus) = entity_to_unfocus {
                                    commands.entity(old_focus).remove::<Focus>();
                                    commands.entity(old_focus).remove::<TextSelection>(); // Remove selection from old entity
                                    info!("Focus lost: {:?}", old_focus);
                                }
                                commands.entity(target_text_entity).insert(Focus);
                                // CursorState is added by manage_cursor_visual_system, update it here
                                commands.entity(target_text_entity).insert(CursorState { position: new_cursor_pos, line: new_cursor_line });
                                // Insert new collapsed selection
                                commands.entity(target_text_entity).insert(TextSelection { start: new_cursor_pos, end: new_cursor_pos });
                                info!("Focus gained: {:?}, CursorState set: pos {}, line {}. Selection set: [{}, {}]", target_text_entity, new_cursor_pos, new_cursor_line, new_cursor_pos, new_cursor_pos);
                                focus_event_to_send = Some(Some(target_text_entity));
                            } else {
                                // Clicked on already focused text entity
                                // Update CursorState
                                commands.entity(target_text_entity).insert(CursorState { position: new_cursor_pos, line: new_cursor_line });
                                // Update selection to collapsed range at click position
                                if let Ok(mut selection) = text_selection_query.get_mut(target_text_entity) {
                                    selection.start = new_cursor_pos;
                                    selection.end = new_cursor_pos;
                                    info!("Focus maintained: {:?}, CursorState updated: pos {}, line {}. Selection updated: [{}, {}]", target_text_entity, new_cursor_pos, new_cursor_line, new_cursor_pos, new_cursor_pos);
                                } else {
                                    // Selection component might not exist if focus was just gained, insert it
                                    commands.entity(target_text_entity).insert(TextSelection { start: new_cursor_pos, end: new_cursor_pos });
                                     info!("Focus maintained: {:?}, CursorState updated: pos {}, line {}. Selection inserted: [{}, {}]", target_text_entity, new_cursor_pos, new_cursor_line, new_cursor_pos, new_cursor_pos);
                                }
                                // No focus change event needed here
                            }
                        } else {
                            // Clicked on non-text or empty space
                            if let Some(old_focus) = entity_to_unfocus {
                                commands.entity(old_focus).remove::<Focus>();
                                commands.entity(old_focus).remove::<TextSelection>(); // Remove selection on focus loss
                                // CursorState is removed automatically by manage_cursor_visual_system
                                info!("Focus lost: {:?}", old_focus);
                                focus_event_to_send = Some(None);
                            }
                        }
                        // Send the event if focus actually changed state
                        if let Some(event_data) = focus_event_to_send {
                            text_focus_writer.send(TextFocusChanged { entity: event_data });
                        }

                        // Clear shape drag state if not clicking on a draggable shape
                        if mouse_context.context != MouseContextType::DraggingShape {
                            *drag_start_position = None;
                            *dragged_entity = None;
                        }
                        // Note: TextInteraction context is set, but drag state isn't initiated yet.
                        // drag_start_position and dragged_entity remain None for text clicks here.

                    }
                }
                ButtonState::Released => {
                    // --- Handle Click Release (Potential Drag End) ---
                    *drag_start_position = None;
                    *dragged_entity = None;
                }
            }
        }
    }

    // --- Process Mouse Motion Events (Dragging Shapes Only for now) ---
    if mouse_context.context == MouseContextType::DraggingShape {
        if let (Some(drag_entity), Some(start_pos)) = (*dragged_entity, *drag_start_position) {
            let mut last_cursor_pos = start_pos;

            // Use read().last() to get the most recent position if multiple events occurred
            if let Some(cursor_pos_window) = cursor_moved_events.read().last().map(|e| e.position) {
                 last_cursor_pos = Vec2::new(cursor_pos_window.x, window_height - cursor_pos_window.y);
            }

            let delta = last_cursor_pos - start_pos;

            if delta.length_squared() > 0.0 {
                entity_dragged_writer.send(EntityDragged {
                    entity: drag_entity,
                    delta,
                });
                // Update start position for next frame's delta calculation
                *drag_start_position = Some(last_cursor_pos);
            }
        }
    } else {
        // If not dragging a shape, clear motion events to avoid processing them next frame
        cursor_moved_events.clear();
    }
    // Note: Text drag selection logic will be added in Phase 3
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