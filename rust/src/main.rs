// /mnt/c/Users/jerem/Desktop/Studio-Whip/rust/src/main.rs
use bevy_app::{App, AppExit, Startup, Update, Last, PluginGroup}; // Added PluginGroup
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::common_conditions::on_event;
use bevy_log::{info, error, warn, LogPlugin, Level};
use bevy_utils::default;
use bevy_math::Vec2; // Use Vec2 from bevy_math
use bevy_input::{InputPlugin, keyboard::KeyCode, mouse::MouseButton, ButtonInput};
use bevy_window::{
    PrimaryWindow, Window, WindowPlugin, WindowCloseRequested, PresentMode,
    WindowResolution, CursorMoved, // Added CursorMoved
};
use bevy_winit::{WinitPlugin, WinitWindows, WakeUp};
use bevy_a11y::AccessibilityPlugin;
use bevy_transform::prelude::{Transform, GlobalTransform}; // Use Bevy's Transform
use bevy_transform::TransformPlugin; // Add TransformPlugin

// Import framework components (Vulkan backend + New ECS parts)
use rusty_whip::gui_framework::{
    VulkanContext,
    context::vulkan_setup::{setup_vulkan, cleanup_vulkan},
    interaction::hotkeys::{HotkeyConfig, HotkeyError}, // Keep for resource loading
    // Import new components and events
    components::{ShapeData, Visibility, Interaction},
    events::{EntityClicked, EntityDragged, HotkeyActionTriggered},
};
use rusty_whip::Vertex; // Keep Vertex

use std::sync::{Arc, Mutex};
use std::any::Any;
// Removed HashMap import (no longer needed for ClickRouter)
use std::path::PathBuf; // Keep for hotkey loading path
use std::env; // Keep for hotkey loading path
use ash::vk;

// --- Bevy Resources ---
#[derive(Resource, Clone)]
struct VulkanContextResource(Arc<Mutex<VulkanContext>>);

// Placeholder struct for data passed to the custom renderer
#[derive(Debug)] // Add Debug for logging if needed
struct RenderCommandData {
    // Define fields based on what the Vulkan renderer will need
    // Example fields (adjust as necessary):
    entity_id: Entity, // Keep track of the source entity
    transform: bevy_math::Mat4, // Pass the full transform matrix
    vertices: Arc<Vec<Vertex>>, // Share vertex data via Arc potentially
    shader_paths: (String, String),
    // depth: f32, // Depth is often handled by sorting before this struct
    // Add instancing info later
}

// --- Placeholder Renderer (Still needed for bridge) ---
// NOTE: This Renderer struct needs to be the *actual* custom Vulkan renderer eventually.
// For now, the placeholder allows compilation.
struct PlaceholderRenderer;
impl PlaceholderRenderer {
    fn new(_vk_ctx: &VulkanContext, _extent: vk::Extent2D) -> Self {
        info!("PlaceholderRenderer::new called");
        Self
    }
    fn render(&mut self, _vk_ctx: &VulkanContext) {
        // info!("PlaceholderRenderer::render called");
    }
    fn resize_renderer(&mut self, _vk_ctx: &VulkanContext, _width: u32, _height: u32) {
         info!("PlaceholderRenderer::resize_renderer called");
    }

    // Change signature from `fn cleanup(self, ...)` to `fn cleanup(&mut self, ...)`
    fn cleanup(&mut self, _vk_ctx: &VulkanContext) { // <-- Changed to &mut self
         info!("PlaceholderRenderer::cleanup called (&mut self version)");
    }
}
// --- End Placeholder Renderer ---

#[derive(Resource, Clone)]
struct RendererResource(Arc<Mutex<PlaceholderRenderer>>); // Still using placeholder

// --- Hotkey Configuration Resource ---
#[derive(Resource, Debug, Clone, Default)]
struct HotkeyResource(HotkeyConfig);


// --- Removed Old Resources/Handlers ---
// #[derive(Resource, Clone)] struct SceneResource(...);
// #[derive(Resource, Clone)] struct EventBusResource(...);
// #[derive(Resource, Clone)] struct InteractionControllerResource(...);
// #[derive(Resource, Clone)] struct ClickRouterResource(...);
// struct ClickRouter { ... }
// impl EventHandler for ClickRouter { ... }
// struct SceneEventHandler { ... }
// impl EventHandler for SceneEventHandler { ... }
// struct HotkeyActionHandler { ... }
// impl EventHandler for HotkeyActionHandler { ... }


fn main() {
    info!("Starting Rusty Whip with Bevy ECS integration (Bevy 0.15)...");

    // --- Initialize Vulkan Context (Remains the same) ---
    let vulkan_context = Arc::new(Mutex::new(VulkanContext::new()));

    // --- Build Bevy App ---
    App::new()
        // == Plugins ==
        // Use MinimalPlugins to avoid Bevy rendering, add necessary core plugins manually
        .add_plugins((
            LogPlugin { level: Level::INFO, filter: "wgpu=error,naga=warn,bevy_app=info,bevy_ecs=info,rusty_whip=debug".to_string(), ..default() },
            bevy_time::TimePlugin::default(),
            TransformPlugin::default(), // Add TransformPlugin
            InputPlugin::default(), // Add InputPlugin explicitly
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
        .insert_resource(VulkanContextResource(vulkan_context))
        // HotkeyResource inserted by setup_scene_ecs
        // RendererResource inserted by create_renderer_system

        // == Events ==
        .add_event::<EntityClicked>()
        .add_event::<EntityDragged>()
        .add_event::<HotkeyActionTriggered>()

        // == Startup Systems ==
        .add_systems(Startup,
            (
                // Setup Vulkan -> Create Renderer -> Setup ECS Scene
                setup_vulkan_system
                    .pipe(create_renderer_system) // Create renderer after Vulkan setup
                    .pipe(setup_scene_ecs), // Setup ECS scene after renderer resource exists
            ).chain()
        )
        // == Update Systems ==
        .add_systems(Update,
            (
                // Input and interaction processing
                interaction_system, // New system for mouse interaction
                hotkey_system, // New system for keyboard hotkeys
                movement_system, // New system to apply drag movements
                // Window and App control
                handle_close_request, // Keep this
                handle_resize_system, // Keep this (but simplified)
                app_control_system, // New system for app exit via hotkey
            )
        )
        // == Rendering System (runs late) ==
        .add_systems(Last, rendering_system) // Renamed from render_trigger_system

        // == Shutdown System ==
        .add_systems(Last, cleanup_system.run_if(on_event::<AppExit>))

        // == Run the App ==
        .run();
}

// --- Bevy Systems ---

/// Startup system: Initializes Vulkan using the primary window handle. (No changes needed)
fn setup_vulkan_system(
    vk_context_res: Res<VulkanContextResource>,
    primary_window_q: Query<Entity, With<PrimaryWindow>>,
    winit_windows: NonSend<WinitWindows>,
) -> Result<(), String> {
    info!("Running setup_vulkan_system...");
    let primary_entity = primary_window_q.get_single()
        .map_err(|e| format!("Failed to get primary window entity: {}", e))?;
    let winit_window = winit_windows.get_window(primary_entity)
        .ok_or_else(|| "Failed to get winit window reference from WinitWindows".to_string())?;
    match vk_context_res.0.lock() {
        Ok(mut vk_ctx_guard) => {
            setup_vulkan(&mut vk_ctx_guard, winit_window);
            info!("Vulkan setup complete.");
            Ok(())
        }
        Err(e) => {
            let err_msg = format!("Failed to lock VulkanContext mutex for setup: {}", e);
            error!("{}", err_msg);
            Err(err_msg)
        }
    }
}

/// Startup system (piped): Creates the Renderer instance resource.
fn create_renderer_system(
    In(setup_result): In<Result<(), String>>, // Get result from previous system
    mut commands: Commands,
    vk_context_res: Res<VulkanContextResource>,
    // Removed SceneResource, EventBusResource
    primary_window_q: Query<&Window, With<PrimaryWindow>>,
) -> Result<(), String> { // Return Result for piping
    if let Err(e) = setup_result {
        error!("Skipping renderer creation due to Vulkan setup error: {}", e);
        return Err(e); // Propagate error
    }
    info!("Running create_renderer_system...");

    let primary_window = primary_window_q.get_single().expect("Primary window not found");
    let extent = vk::Extent2D { width: primary_window.physical_width(), height: primary_window.physical_height() };

    let vk_ctx_guard = vk_context_res.0.lock().expect("Failed to lock VulkanContext");

    // Create the placeholder renderer instance
    let renderer_instance = PlaceholderRenderer::new(&vk_ctx_guard, extent);
    warn!("Using placeholder Renderer creation.");

    // Wrap in Arc<Mutex> and insert as resource
    let renderer_arc = Arc::new(Mutex::new(renderer_instance));
    commands.insert_resource(RendererResource(renderer_arc.clone()));

    // Removed event bus subscription

    info!("Renderer resource created (using placeholder).");
    Ok(()) // Indicate success
}

/// Startup system (piped): Loads hotkeys and spawns initial ECS entities.
fn setup_scene_ecs(
    In(renderer_result): In<Result<(), String>>, // Get result from previous system
    mut commands: Commands,
    // Add other resources if needed (e.g., AssetServer for shaders later)
) {
     if let Err(e) = renderer_result {
        error!("Skipping ECS scene setup due to previous error: {}", e);
        return;
    }
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
    let width = 600.0;
    let height = 300.0;

    // Background (Not interactive, covers full screen)
    commands.spawn((
        ShapeData {
            vertices: vec![
                Vertex { position: [0.0, 0.0] },
                Vertex { position: [0.0, height] },
                Vertex { position: [width, height] },
                Vertex { position: [width, 0.0] },
            ],
            vertex_shader_path: "background.vert.spv".to_string(),
            fragment_shader_path: "background.frag.spv".to_string(),
        },
        Transform::from_xyz(0.0, 0.0, 0.0), // Use Z for depth
        Visibility(true), // Use custom Visibility component
        Interaction::default(), // Not interactive
        Name::new("Background"), // Optional: Add bevy_core::Name for debugging
    ));

    // Triangle (Draggable and Clickable)
    commands.spawn((
        ShapeData {
            vertices: vec![
                Vertex { position: [-25.0, -25.0] }, // Centered around (0,0)
                Vertex { position: [0.0, 25.0] },
                Vertex { position: [25.0, -25.0] },
            ],
            vertex_shader_path: "triangle.vert.spv".to_string(),
            fragment_shader_path: "triangle.frag.spv".to_string(),
        },
        Transform::from_xyz(300.0, 150.0, 1.0), // Positioned in center, depth 1
        Visibility(true),
        Interaction { clickable: true, draggable: true },
        Name::new("Triangle"),
    ));

    // Square (Draggable and Clickable)
    commands.spawn((
        ShapeData {
            vertices: vec![
                Vertex { position: [-25.0, -25.0] }, // Centered around (0,0)
                Vertex { position: [-25.0, 25.0] },
                Vertex { position: [25.0, 25.0] },
                Vertex { position: [25.0, -25.0] },
            ],
            vertex_shader_path: "square.vert.spv".to_string(),
            fragment_shader_path: "square.frag.spv".to_string(),
        },
        Transform::from_xyz(125.0, 75.0, 2.0), // Positioned, depth 2 (on top)
        Visibility(true),
        Interaction { clickable: true, draggable: true },
        Name::new("Square"),
    ));

    // TODO: Add instancing later using ECS patterns (e.g., marker components, parent/child, custom relations)

    info!("Initial ECS entities spawned.");
}


/// Update system: Handles window resize events. (Simplified)
fn handle_resize_system(
    mut resize_reader: EventReader<bevy_window::WindowResized>,
    // Get resources directly. Use Option<> in case they don't exist yet/failed setup.
    renderer_res_opt: Option<ResMut<RendererResource>>, // Use ResMut if resize_renderer needs &mut
    vk_context_res_opt: Option<Res<VulkanContextResource>>,
    // Removed SceneResource
) {
    let Some(renderer_res) = renderer_res_opt else { return; };
    let Some(vk_context_res) = vk_context_res_opt else { return; };

    for event in resize_reader.read() {
        info!("WindowResized event: {:?}", event);
        if event.width > 0.0 && event.height > 0.0 {
            // Lock resources needed for resize_renderer
            // Still faces the &mut VulkanContext issue for the *actual* renderer.
            if let (Ok(mut renderer_guard), Ok(vk_ctx_guard)) = (
                renderer_res.0.lock(),
                vk_context_res.0.lock(),
            ) {
                warn!("Calling placeholder resize logic.");
                // --- HACK/Placeholder ---
                // Actual call would need refactoring:
                renderer_guard.resize_renderer(&vk_ctx_guard, event.width as u32, event.height as u32);
                // --- End HACK ---
            } else {
                warn!("Could not lock resources for resize handling.");
            }
        }
    }
}

/// Update system: Processes mouse input for clicking and dragging.
fn interaction_system(
    mouse_button_input: Res<ButtonInput<MouseButton>>, // <-- Corrected type
    mut cursor_evr: EventReader<CursorMoved>,
    mut entity_clicked_evw: EventWriter<EntityClicked>,
    mut entity_dragged_evw: EventWriter<EntityDragged>,
    query: Query<(Entity, &Transform, &ShapeData, &Visibility, &Interaction)>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    mut drag_state: Local<Option<(Entity, Vec2)>>,
) {
    let Ok(window) = window_q.get_single() else { return }; // Exit if no primary window
    let window_height = window.height();

    let mut current_cursor_pos: Option<Vec2> = None;
    for ev in cursor_evr.read() {
        current_cursor_pos = Some(ev.position);
    }

    // --- Dragging Logic ---
    if mouse_button_input.pressed(MouseButton::Left) {
        if let Some(cursor_pos) = current_cursor_pos {
            if let Some((dragged_entity, last_pos)) = *drag_state {
                // Continue dragging
                let delta = cursor_pos - last_pos;
                if delta.length_squared() > 0.0 { // Only send event if moved
                    entity_dragged_evw.send(EntityDragged { entity: dragged_entity, delta });
                    // Update last position for next frame's delta calculation
                    *drag_state = Some((dragged_entity, cursor_pos));
                }
            } else {
                // Start dragging? Check if cursor is over a draggable entity.
                // Find the top-most (highest Z) draggable entity under the cursor.
                let mut top_hit: Option<(Entity, f32)> = None; // (Entity, Z-depth)

                for (entity, transform, shape, visibility, interaction) in query.iter() {
                    if !visibility.0 || !interaction.draggable { continue; }

                    // Basic AABB Hit Test (using Transform and ShapeData)
                    // Note: Assumes vertices in ShapeData are relative to origin (0,0)
                    // Note: Ignores rotation/scale for simplicity for now.
                    let pos = transform.translation;
                    let (min_x, max_x, min_y, max_y) = shape.vertices.iter().fold(
                        (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY),
                        |acc, v| (acc.0.min(v.position[0]), acc.1.max(v.position[0]), acc.2.min(v.position[1]), acc.3.max(v.position[1]))
                    );
                    // Adjust AABB by entity's position
                    let world_min_x = min_x + pos.x;
                    let world_max_x = max_x + pos.x;
                    let world_min_y = min_y + pos.y;
                    let world_max_y = max_y + pos.y;

                    // Adjust cursor Y for screen coordinates (Y down) vs world coordinates (Y up)
                    let adjusted_cursor_y = window_height - cursor_pos.y;

                    if cursor_pos.x >= world_min_x && cursor_pos.x <= world_max_x &&
                       adjusted_cursor_y >= world_min_y && adjusted_cursor_y <= world_max_y {
                        // Hit! Check if it's the top-most one so far
                        if top_hit.is_none() || pos.z > top_hit.unwrap().1 {
                            top_hit = Some((entity, pos.z));
                        }
                    }
                }

                // If we found a top-most entity, start dragging it
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
            // Find the top-most *clickable* entity under the cursor.
            let mut top_hit: Option<(Entity, f32)> = None; // (Entity, Z-depth)
            for (entity, transform, shape, visibility, interaction) in query.iter() {
                 if !visibility.0 || !interaction.clickable { continue; } // Check clickable flag

                // Re-do hit test (could be optimized)
                let pos = transform.translation;
                let (min_x, max_x, min_y, max_y) = shape.vertices.iter().fold(
                    (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY), // init
                    |acc, v| (acc.0.min(v.position[0]), acc.1.max(v.position[0]), acc.2.min(v.position[1]), acc.3.max(v.position[1])) // f
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
            // If we hit a clickable entity, send the event
            if let Some((hit_entity, _)) = top_hit {
                entity_clicked_evw.send(EntityClicked { entity: hit_entity });
                info!("Sent EntityClicked event for {:?}", hit_entity); // Debug log
            }
        }
    }
}


/// Update system: Applies movement deltas from drag events to Transforms.
fn movement_system(
    mut drag_evr: EventReader<EntityDragged>,
    mut query: Query<&mut Transform>, // Query for transforms to update
) {
    for ev in drag_evr.read() {
        if let Ok(mut transform) = query.get_mut(ev.entity) {
            // Apply delta, adjusting for coordinate system if necessary
            // Assuming delta is in screen pixels, Y positive down.
            // Transform Y is typically positive up.
            transform.translation.x += ev.delta.x;
            transform.translation.y -= ev.delta.y; // Invert Y delta
        }
    }
}

/// Update system: Detects keyboard input and sends HotkeyActionTriggered events.
fn hotkey_system(
    keyboard_input: Res<ButtonInput<KeyCode>>, // <-- Corrected type
    hotkey_config: Res<HotkeyResource>,
    mut hotkey_evw: EventWriter<HotkeyActionTriggered>,
) {
    // Check for modifiers (adapt format_key_event logic or use bevy_input methods)
    let ctrl = keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let alt = keyboard_input.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);
    let shift = keyboard_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let super_key = keyboard_input.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]); // Windows/Command key

    for keycode in keyboard_input.get_just_pressed() {
        // Format the key combination string (similar to format_key_event)
        let mut parts = Vec::new();
        if ctrl { parts.push("Ctrl"); }
        if alt { parts.push("Alt"); }
        if shift { parts.push("Shift"); }
        if super_key { parts.push("Super"); }

        let key_str = match keycode {
            // Map KeyCode variants to strings matching hotkeys.toml format
            KeyCode::KeyA => "A", KeyCode::KeyB => "B", // ... etc for A-Z
            KeyCode::KeyC => "C", KeyCode::KeyD => "D", KeyCode::KeyE => "E",
            KeyCode::KeyF => "F", KeyCode::KeyG => "G", KeyCode::KeyH => "H",
            KeyCode::KeyI => "I", KeyCode::KeyJ => "J", KeyCode::KeyK => "K",
            KeyCode::KeyL => "L", KeyCode::KeyM => "M", KeyCode::KeyN => "N",
            KeyCode::KeyO => "O", KeyCode::KeyP => "P", KeyCode::KeyQ => "Q",
            KeyCode::KeyR => "R", KeyCode::KeyS => "S", KeyCode::KeyT => "T",
            KeyCode::KeyU => "U", KeyCode::KeyV => "V", KeyCode::KeyW => "W",
            KeyCode::KeyX => "X", KeyCode::KeyY => "Y", KeyCode::KeyZ => "Z",
            KeyCode::Digit0 => "0", KeyCode::Digit1 => "1", // ... etc for 0-9
            KeyCode::Digit2 => "2", KeyCode::Digit3 => "3", KeyCode::Digit4 => "4",
            KeyCode::Digit5 => "5", KeyCode::Digit6 => "6", KeyCode::Digit7 => "7",
            KeyCode::Digit8 => "8", KeyCode::Digit9 => "9",
            KeyCode::F1 => "F1", KeyCode::F2 => "F2", // ... etc for F1-F12
            KeyCode::F3 => "F3", KeyCode::F4 => "F4", KeyCode::F5 => "F5",
            KeyCode::F6 => "F6", KeyCode::F7 => "F7", KeyCode::F8 => "F8",
            KeyCode::F9 => "F9", KeyCode::F10 => "F10", KeyCode::F11 => "F11",
            KeyCode::F12 => "F12",
            KeyCode::Escape => "Escape",
            KeyCode::Space => "Space",
            KeyCode::Enter => "Enter",
            KeyCode::Backspace => "Backspace",
            KeyCode::Delete => "Delete",
            KeyCode::Tab => "Tab",
            KeyCode::ArrowUp => "ArrowUp", KeyCode::ArrowDown => "ArrowDown",
            KeyCode::ArrowLeft => "ArrowLeft", KeyCode::ArrowRight => "ArrowRight",
            _ => continue, // Ignore keys not used in hotkeys.toml format
        };
        parts.push(key_str);
        let key_combo_str = parts.join("+");

        // Check if this combo is mapped in the config
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
            app_exit_evw.send(AppExit::Success); // Use AppExit::Success or AppExit::Error as appropriate
        }
        // Handle other app-level actions here
    }
}


/// Update system: Triggers rendering via the custom Vulkan Renderer. (Modified)
fn rendering_system(
    renderer_res_opt: Option<Res<RendererResource>>,
    vk_context_res_opt: Option<Res<VulkanContextResource>>,
    // Query for renderable entities using ECS components
    query: Query<(&Transform, &ShapeData, &Visibility)>,
) {
    if let (Some(renderer_res), Some(vk_context_res)) =
        (renderer_res_opt, vk_context_res_opt)
    {
        // --- Collect Render Data from ECS ---
        let mut render_commands: Vec<RenderCommandData> = Vec::new();
        for (transform, shape, visibility) in query.iter() {
            if visibility.0 { // Check custom visibility component
                // TODO: Define a RenderCommandData struct
                // struct RenderCommandData {
                //     transform: Mat4, // Or relevant parts like position, scale, rotation
                //     vertices: Arc<Vec<Vertex>>, // Or some handle/ID
                //     shader_paths: (String, String),
                //     depth: f32,
                //     // Add instancing info later
                // }
                // Populate RenderCommandData from components
                // render_commands.push(RenderCommandData { ... });
            }
        }
        // TODO: Sort render_commands by depth (transform.translation.z)

        // --- Call Custom Renderer ---
        if let (Ok(mut renderer_guard), Ok(vk_ctx_guard)) = (
            renderer_res.0.lock(),
            vk_context_res.0.lock(),
        ) {
            // --- HACK/Placeholder ---
            // Actual call needs refactor:
            // renderer_guard.render(&vk_ctx_guard, &render_commands); // Pass collected data
            // Placeholder call (doesn't use ECS data yet):
            renderer_guard.render(&vk_ctx_guard);
            // --- End HACK ---
        } else {
            warn!("Could not lock resources for rendering trigger.");
        }
    }
}

/// Update system: Handles WindowCloseRequested events -> AppExit. (No changes needed)
fn handle_close_request(
    mut ev_close: EventReader<WindowCloseRequested>,
    mut ev_app_exit: EventWriter<AppExit>,
) {
    if ev_close.read().next().is_some() {
        info!("WindowCloseRequested detected, sending AppExit.");
        ev_app_exit.send(AppExit::Success);
    }
}

/// System running on AppExit: Cleans up resources. (Simplified)
fn cleanup_system(
    mut commands: Commands,
    renderer_res_opt: Option<ResMut<RendererResource>>, // Request mutable access
    vk_context_res_opt: Option<Res<VulkanContextResource>>,
) {
    info!("Running cleanup_system...");

    let Some(vk_context_res) = vk_context_res_opt else {
        warn!("VulkanContext resource not found for cleanup.");
        return;
    };

    // 1. Attempt to cleanup Renderer via Mutex lock
    if let Some(mut renderer_res_mut) = renderer_res_opt { // Get ResMut
        info!("RendererResource found.");

        // Lock the mutex within the resource
        match renderer_res_mut.0.lock() {
            Ok(mut renderer_guard) => { // Get MutexGuard<PlaceholderRenderer> - guard is mutable
                info!("Successfully locked Renderer Mutex.");
                // Lock Vulkan context needed for cleanup call
                if let Ok(vk_ctx_guard) = vk_context_res.0.lock() {
                    warn!("Calling placeholder Renderer cleanup via MutexGuard.");
                    // Call cleanup on the mutable reference from the guard
                    renderer_guard.cleanup(&vk_ctx_guard); // Call cleanup(&mut self, ...)

                } else {
                    error!("Could not lock VulkanContext for Renderer cleanup step.");
                }
            }
            Err(poisoned) => {
                error!("Renderer Mutex was poisoned before cleanup: {:?}", poisoned);
                // Handle poisoned mutex if necessary
            }
        }
        // Signal that the resource should be removed *after* this system runs
        commands.remove_resource::<RendererResource>();
        info!("Signaled removal of RendererResource.");

    } else {
        info!("Renderer resource not found for cleanup (already removed or never inserted?).");
    }

    // 2. Cleanup Vulkan Context
    if let Ok(mut vk_ctx_guard) = vk_context_res.0.lock() {
        info!("Calling cleanup_vulkan...");
        cleanup_vulkan(&mut vk_ctx_guard);
        info!("cleanup_vulkan finished.");
    } else {
        error!("Could not lock VulkanContext for final cleanup.");
    }

    info!("Cleanup complete.");
}


// Helper to re-add Name component if needed (requires bevy_core feature)
use bevy_core::Name; // Add this use statement if using Name component