use bevy_app::{App, AppExit, Startup, Update, Last};
use bevy_ecs::prelude::*;
use std::collections::HashSet;
use bevy_ecs::schedule::common_conditions::not;
use bevy_ecs::schedule::common_conditions::on_event;
use bevy_log::{info, error, warn, LogPlugin, Level};
use bevy_utils::default;
use bevy_math::Vec2;
use bevy_input::{InputPlugin, keyboard::KeyCode, mouse::MouseButton, ButtonInput};
use bevy_window::{
    PrimaryWindow, Window, WindowPlugin, WindowCloseRequested, PresentMode,
    WindowResolution, CursorMoved,
};
use bevy_winit::{WinitPlugin, WinitWindows, WakeUp};
use bevy_a11y::AccessibilityPlugin;
use bevy_transform::prelude::{Transform, GlobalTransform};
use bevy_transform::TransformPlugin;
use bevy_reflect::Reflect;
// Import types defined in lib.rs
use rusty_whip::{Vertex, RenderCommandData, VulkanContextResource, RendererResource};

use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::env;
use ash::vk;
// Import framework components, events, resources, and the new plugin
use rusty_whip::gui_framework::{
    VulkanContext,
    interaction::hotkeys::{HotkeyConfig, HotkeyError},
    components::{ShapeData, Visibility, Interaction},
    events::{EntityClicked, EntityDragged, HotkeyActionTriggered},
    plugins::core::GuiFrameworkCorePlugin,
};
// Import Bevy Name for debugging entities
use bevy_core::Name;


#[derive(Component)]
struct BackgroundQuad;

// --- Bevy Resources ---
// Defined in lib.rs now
// #[derive(Resource, Clone)]
// struct VulkanContextResource(Arc<Mutex<VulkanContext>>);
// #[derive(Resource, Clone)]
// struct RendererResource(Arc<Mutex<Renderer>>);

// --- Hotkey Configuration Resource ---
#[derive(Resource, Debug, Clone, Default, Reflect)]
struct HotkeyResource(HotkeyConfig);

fn main() {
    info!("Starting Rusty Whip with Bevy ECS integration (Bevy 0.15)...");

    // --- Initialize Vulkan Context ---
    let vulkan_context = Arc::new(Mutex::new(VulkanContext::new()));

    // --- Build Bevy App ---
    App::new()
        .add_plugins((
            LogPlugin { level: Level::INFO, filter: "wgpu=error,naga=warn,bevy_app=info,bevy_ecs=info,rusty_whip=debug".to_string(), ..default() },
            bevy_time::TimePlugin::default(),
            TransformPlugin::default(),
            InputPlugin::default(),
            WindowPlugin {
                primary_window: Some(Window {
                    title: "Rusty Whip (Bevy ECS Migration)".into(),
                    resolution: WindowResolution::new(600.0, 300.0),
                    present_mode: PresentMode::AutoVsync,
                    ..default()
                }),
                ..default()
            },
            AccessibilityPlugin,
            WinitPlugin::<WakeUp>::default(),
            // DO NOT ADD DefaultPlugins, RenderPlugin, PbrPlugin, etc.
        ))
        // == Resources ==
        .insert_resource(VulkanContextResource(vulkan_context)) // Insert Vulkan context here
        // HotkeyResource inserted by setup_scene_ecs
        // RendererResource inserted by GuiFrameworkCorePlugin's create_renderer_system

        // == Events ==
        .add_event::<EntityClicked>()
        .add_event::<EntityDragged>()
        .add_event::<HotkeyActionTriggered>()
        // == Reflection Registration ==
        // Core types (Vertex, ShapeData, Visibility) registered by GuiFrameworkCorePlugin
        // Register app-specific / interaction components & events here
        .register_type::<Interaction>() 
        .register_type::<EntityClicked>()
        .register_type::<EntityDragged>()
        .register_type::<HotkeyActionTriggered>()
        .register_type::<HotkeyResource>()
        .register_type::<HotkeyConfig>() 

        // == Add Core Framework Plugin ==
        .add_plugins(GuiFrameworkCorePlugin)

        // == Startup Systems ==
        .add_systems(Startup,
            (
                setup_scene_ecs, // Setup app-specific ECS scene (runs after plugin setup)
            )//.after(GuiFrameworkCorePlugin::StartupSystems)) // TODO: Use System Sets later
        )
        // == Update Systems ==
        .add_systems(Update,
            (
                // Input and interaction processing (App specific)
                background_resize_system, // App specific background handling
                interaction_system,
                hotkey_system,
                movement_system,
                // Window and App control (App specific)
                handle_close_request,
                app_control_system, // for app exit via hotkey
            ) // Removed .chain()
        )
        // == Rendering System (runs late) ==
        // Moved to GuiFrameworkCorePlugin

        // == Shutdown System ==
        // Moved to GuiFrameworkCorePlugin

        // == Run the App ==
        .run();
}

// --- Bevy Systems ---

/// Startup system: Loads hotkeys and spawns initial ECS entities.
fn setup_scene_ecs(
    mut commands: Commands,
    primary_window_q: Query<&Window, With<PrimaryWindow>>,
) {
    // if let Err(e) = renderer_result { // No longer piped
    //    error!("Skipping ECS scene setup due to previous error: {}", e);
    //    return;
    //}
    info!("Running setup_scene_ecs...");

    // --- Load Hotkey Configuration ---
    let mut hotkey_path: Option<PathBuf> = None;
    let mut config_load_error: Option<String> = None;
    match env::current_exe() {
        Ok(mut exe_path) => {
            if exe_path.pop() {
                let path = exe_path.join("user").join("hotkeys.toml");
                info!("[HotkeyLoader] Looking for hotkeys file at: {:?}", path);
                hotkey_path = Some(path);
            } else { config_load_error = Some("Could not get executable directory.".to_string()); }
        }
        Err(e) => { config_load_error = Some(format!("Failed to get current executable path: {}", e)); }
    }
    let config = match hotkey_path {
        Some(path) => {
            if path.exists() {
                HotkeyConfig::load_config(&path).unwrap_or_else(|e| {
                    match e {
                        HotkeyError::ReadError(io_err) => error!("[HotkeyLoader] Error reading hotkey file '{:?}': {}", path, io_err),
                        HotkeyError::ParseError(toml_err) => error!("[HotkeyLoader] Error parsing hotkey file '{:?}': {}", path, toml_err),
                        HotkeyError::FileNotFound(_) => error!("[HotkeyLoader] Hotkey file disappeared between check and load: {:?}", path), // Should not happen
                    }
                    HotkeyConfig::default()
                })
            } else {
                warn!("[HotkeyLoader] Hotkey file '{:?}' not found. Using default empty configuration.", path);
                HotkeyConfig::default()
            }
        }
        None => {
            error!("[HotkeyLoader] {}", config_load_error.unwrap_or("Hotkey path could not be determined.".to_string()));
            HotkeyConfig::default()
        }
    };
    commands.insert_resource(HotkeyResource(config));
    info!("Hotkey configuration loaded and inserted as resource.");


    // --- Spawn Initial Entities ---
    let Ok(primary_window) = primary_window_q.get_single() else {
        error!("Primary window not found in setup_scene_ecs!");
        return; // Or handle error appropriately
   };
   let logical_width = primary_window.width();
   let logical_height = primary_window.height(); 

   info!("Using logical dimensions for background: {}x{}", logical_width, logical_height);

    // Background (Not interactive, covers full screen)
    commands.spawn((
        ShapeData {
            vertices: Arc::new(vec![
                // Triangle 1
                Vertex { position: [0.0, 0.0] },   // Bottom-left (0,0)
                Vertex { position: [0.0, logical_height] }, // Top-left (0, H)
                Vertex { position: [logical_width, 0.0] },  // Bottom-right (W, 0)
                // Triangle 2
                Vertex { position: [logical_width, 0.0] },  // Bottom-right (W, 0)
                Vertex { position: [0.0, logical_height] }, // Top-left (0, H)
                Vertex { position: [logical_width, logical_height] },// Top-right (W, H)
            ]),
            vertex_shader_path: "background.vert.spv".to_string(),
            fragment_shader_path: "background.frag.spv".to_string(),
        },
        Transform::from_xyz(0.0, 0.0, 0.0), // Use Z for depth
        Visibility(true),
        Interaction::default(), // Not interactive
        Name::new("Background"), // Optional: Add bevy_core::Name for debugging
        BackgroundQuad,
    ));

    // Triangle (Draggable and Clickable)
    commands.spawn((
        ShapeData {
            vertices: Arc::new(vec![ // Use Arc
                Vertex { position: [-25.0, -25.0] }, // Bottom-left local
                Vertex { position: [0.0, 25.0] },    // Top-center local
                Vertex { position: [25.0, -25.0] }, // Bottom-right local
            ]),
            vertex_shader_path: "triangle.vert.spv".to_string(),
            fragment_shader_path: "triangle.frag.spv".to_string(),
        },
        Transform::from_xyz(300.0, 150.0, 1.0),
        Visibility(true),
        Interaction { clickable: true, draggable: true },
        Name::new("Triangle"),
    ));

    // Square (Draggable and Clickable)
    commands.spawn((
        ShapeData {
            vertices: Arc::new(vec![ // Use Arc
                Vertex { position: [-25.0, -25.0] }, // Bottom-left local
                Vertex { position: [-25.0, 25.0] },  // Top-left local
                Vertex { position: [25.0, -25.0] },  // Bottom-right local
                // Triangle 2
                Vertex { position: [25.0, -25.0] },  // Bottom-right local
                Vertex { position: [-25.0, 25.0] },  // Top-left local
                Vertex { position: [25.0, 25.0] },   // Top-right local
            ]),
            vertex_shader_path: "square.vert.spv".to_string(),
            fragment_shader_path: "square.frag.spv".to_string(),
        },
        Transform::from_xyz(125.0, 75.0, 2.0),
        Visibility(true),
        Interaction { clickable: true, draggable: true },
        Name::new("Square"),
    ));

    // TODO: Add instancing later using ECS patterns

    info!("Initial ECS entities spawned.");
}

// System to update background vertices on resize
fn background_resize_system(
    mut resize_reader: EventReader<bevy_window::WindowResized>,
    // Query for the background entity's ShapeData, identified by BackgroundQuad marker
    mut background_query: Query<&mut ShapeData, With<BackgroundQuad>>,
) {
    // Iterate through resize events (usually just one per frame)
    for event in resize_reader.read() {
        // Use the logical width/height from the event
        if event.width > 0.0 && event.height > 0.0 {
            let logical_width = event.width;
            let logical_height = event.height;
            info!("Window resized to logical: {}x{}. Updating background vertices.", logical_width, logical_height);

            // Try to get the background entity's ShapeData
            if let Ok(mut shape_data) = background_query.get_single_mut() {
                // Recalculate vertices based on new logical size
                // Use Arc::make_mut to get a mutable reference if the Arc is unique,
                // or clone if it might be shared (safer default). Cloning is fine here.
                // Or simply replace the Arc entirely. Replacing is simplest:
                shape_data.vertices = Arc::new(vec![
                    Vertex { position: [0.0, 0.0] },
                    Vertex { position: [0.0, logical_height] },
                    Vertex { position: [logical_width, 0.0] },
                    Vertex { position: [logical_width, 0.0] },
                    Vertex { position: [0.0, logical_height] },
                    Vertex { position: [logical_width, logical_height] },
                ]);
                // The Changed<ShapeData> detection will handle buffer updates
            } else {
                // This might happen briefly during shutdown or if setup failed
                warn!("BackgroundQuad entity ShapeData not found during resize update.");
            }
        }
    }
}

// Update system: Processes mouse input for clicking and dragging.
fn interaction_system(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut cursor_evr: EventReader<CursorMoved>,
    mut entity_clicked_evw: EventWriter<EntityClicked>,
    mut entity_dragged_evw: EventWriter<EntityDragged>,
    query: Query<(Entity, &Transform, &ShapeData, &Visibility, &Interaction)>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    mut drag_state: Local<Option<(Entity, Vec2)>>,
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
            if let Some((dragged_entity, last_pos)) = *drag_state {
                let delta = cursor_pos - last_pos;
                if delta.length_squared() > 0.0 {
                    entity_dragged_evw.send(EntityDragged { entity: dragged_entity, delta });
                    *drag_state = Some((dragged_entity, cursor_pos));
                }
            } else {
                let mut top_hit: Option<(Entity, f32)> = None;
                for (entity, transform, shape, visibility, interaction) in query.iter() {
                    if !visibility.0 || !interaction.draggable { continue; }
                    let pos = transform.translation;
                    // Use Arc'd vertices
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
                if let Some((hit_entity, _)) = top_hit {
                    *drag_state = Some((hit_entity, cursor_pos));
                }
            }
        }
    }

    // --- Stop Dragging ---
    if mouse_button_input.just_released(MouseButton::Left) {
        *drag_state = None;
    }

    // --- Clicking Logic ---
    if mouse_button_input.just_pressed(MouseButton::Left) {
        if let Some(cursor_pos) = current_cursor_pos {
            let mut top_hit: Option<(Entity, f32)> = None;
            for (entity, transform, shape, visibility, interaction) in query.iter() {
                 if !visibility.0 || !interaction.clickable { continue; }
                let pos = transform.translation;
                // Use Arc'd vertices
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
            if let Some((hit_entity, _)) = top_hit {
                entity_clicked_evw.send(EntityClicked { entity: hit_entity });
                info!("Sent EntityClicked event for {:?}", hit_entity);
            }
        }
    }
}


// Update system: Applies movement deltas from drag events to Transforms.
fn movement_system(
    mut drag_evr: EventReader<EntityDragged>,
    mut query: Query<&mut Transform>,
) {
    for ev in drag_evr.read() {
        if let Ok(mut transform) = query.get_mut(ev.entity) {
            transform.translation.x += ev.delta.x;
            // Apply Y delta directly (Y-up world coordinates)
            transform.translation.y -= ev.delta.y;
        }
    }
}


// Update system: Detects keyboard input and sends HotkeyActionTriggered events.
fn hotkey_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    hotkey_config: Res<HotkeyResource>,
    mut hotkey_evw: EventWriter<HotkeyActionTriggered>,
) {
    let ctrl = keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let alt = keyboard_input.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);
    let shift = keyboard_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let super_key = keyboard_input.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]);

    for keycode in keyboard_input.get_just_pressed() {
        let mut parts = Vec::new();
        if ctrl { parts.push("Ctrl"); }
        if alt { parts.push("Alt"); }
        if shift { parts.push("Shift"); }
        if super_key { parts.push("Super"); }

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
            _ => continue,
        };
        parts.push(key_str);
        let key_combo_str = parts.join("+");

        if let Some(action) = hotkey_config.0.get_action(&key_combo_str) {
            info!("Hotkey detected: {} -> Action: {}", key_combo_str, action);
            hotkey_evw.send(HotkeyActionTriggered { action: action.clone() });
        }
    }
}

/// Update system: Handles application control actions (e.g., exit).
fn app_control_system(
    mut hotkey_evr: EventReader<HotkeyActionTriggered>,
    mut app_exit_evw: EventWriter<AppExit>,
) {
    for ev in hotkey_evr.read() {
        if ev.action == "CloseRequested" {
            info!("'CloseRequested' hotkey action received, sending AppExit.");
            app_exit_evw.send(AppExit::Success);
        }
    }
}

// Update system: Handles WindowCloseRequested events -> AppExit.
fn handle_close_request(
    mut ev_close: EventReader<WindowCloseRequested>,
    mut ev_app_exit: EventWriter<AppExit>,
) {
    if ev_close.read().next().is_some() {
        info!("WindowCloseRequested detected, sending AppExit.");
        ev_app_exit.send(AppExit::Success);
    }
}

// Systems moved to GuiFrameworkCorePlugin:
// - setup_vulkan_system
// - create_renderer_system
// - handle_resize_system
// - rendering_system
// - cleanup_trigger_system