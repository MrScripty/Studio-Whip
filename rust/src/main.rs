use bevy_app::{App, AppExit, Startup, Update, Last};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::common_conditions::not; // Import 'not' condition
use bevy_ecs::schedule::common_conditions::on_event; // Import 'on_event' condition
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
use bevy_core::Name; // Add Name for debugging

// Import framework components (Vulkan backend + New ECS parts)
use rusty_whip::gui_framework::{
    VulkanContext,
    context::vulkan_setup::{setup_vulkan, cleanup_vulkan},
    interaction::hotkeys::{HotkeyConfig, HotkeyError}, // Keep for resource loading
    // Import new components and events
    components::{ShapeData, Visibility, Interaction},
    events::{EntityClicked, EntityDragged, HotkeyActionTriggered},
    // Import the actual Renderer now
    rendering::render_engine::Renderer, // <-- Use actual Renderer
};
// Import types defined in lib.rs
use rusty_whip::{Vertex, RenderCommandData}; // <-- Import RenderCommandData from lib

use std::sync::{Arc, Mutex};
// Removed Any import
use std::path::PathBuf; // Keep for hotkey loading path
use std::env; // Keep for hotkey loading path
use ash::vk;

// --- Bevy Resources ---
#[derive(Resource, Clone)]
struct VulkanContextResource(Arc<Mutex<VulkanContext>>);

#[derive(Resource, Clone)]
struct RendererResource(Arc<Mutex<Renderer>>);

// --- Hotkey Configuration Resource ---
#[derive(Resource, Debug, Clone, Default, Reflect)]
struct HotkeyResource(HotkeyConfig);

fn main() {
    info!("Starting Rusty Whip with Bevy ECS integration (Bevy 0.15)...");

    // --- Initialize Vulkan Context (Remains the same) ---
    let vulkan_context = Arc::new(Mutex::new(VulkanContext::new()));

    // --- Build Bevy App ---
    App::new()
        // == Plugins ==
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
        // == Reflection Registration ==
        // Register components
        .register_type::<Interaction>()
        .register_type::<ShapeData>() // Requires Vertex: Reflect + TypePath in lib.rs
        .register_type::<Visibility>()
        // Register events
        .register_type::<EntityClicked>()
        .register_type::<EntityDragged>()
        .register_type::<HotkeyActionTriggered>()
        // Register resources
        .register_type::<HotkeyResource>()
        .register_type::<HotkeyConfig>() // Also register the inner config struct

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
                // Add the cleanup trigger system here, running if AppExit occurs
                cleanup_trigger_system.run_if(on_event::<AppExit>),
            ).chain() // Ensure cleanup runs after other Update systems if AppExit happens
        )
        // == Rendering System (runs late) ==
        // Run rendering system only if AppExit hasn't been sent this frame
        .add_systems(Last, rendering_system.run_if(not(on_event::<AppExit>)))

        // == Shutdown System ==
        // Removed cleanup_system registration from Last schedule

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
            // Pass a mutable reference to the VulkanContext inside the MutexGuard
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
    primary_window_q: Query<&Window, With<PrimaryWindow>>,
) -> Result<(), String> { // Return Result for piping
    if let Err(e) = setup_result {
        error!("Skipping renderer creation due to Vulkan setup error: {}", e);
        return Err(e); // Propagate error
    }
    info!("Running create_renderer_system...");

    let primary_window = primary_window_q.get_single().expect("Primary window not found");
    let extent = vk::Extent2D { width: primary_window.physical_width(), height: primary_window.physical_height() };

    // Lock the context to pass to Renderer::new
    let mut vk_ctx_guard = vk_context_res.0.lock().expect("Failed to lock VulkanContext for renderer creation");

    // Create the *actual* renderer instance
    let renderer_instance = Renderer::new(&mut vk_ctx_guard, extent); // Pass &mut guard
    info!("Actual Renderer instance created.");

    // Wrap in Arc<Mutex> and insert as resource
    let renderer_arc = Arc::new(Mutex::new(renderer_instance));
    commands.insert_resource(RendererResource(renderer_arc.clone()));

    info!("Renderer resource created.");
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
    // (Hotkey loading logic remains the same)
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
            vertices: Arc::new(vec![ // Use Arc for vertices
                // Triangle 1
                Vertex { position: [0.0, 0.0] },   // Top-left
                Vertex { position: [0.0, height] }, // Bottom-left
                Vertex { position: [width, 0.0] },  // Top-right
                // Triangle 2
                Vertex { position: [width, 0.0] },  // Top-right
                Vertex { position: [0.0, height] }, // Bottom-left
                Vertex { position: [width, height] },// Bottom-right
            ]),
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
            vertices: Arc::new(vec![ // Use Arc
                Vertex { position: [-25.0, -25.0] }, // Bottom-left local
                Vertex { position: [0.0, 25.0] },    // Top-center local
                Vertex { position: [25.0, -25.0] }, // Bottom-right local
            ]),
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
        Transform::from_xyz(125.0, 75.0, 2.0), // Positioned, depth 2 (on top)
        Visibility(true),
        Interaction { clickable: true, draggable: true },
        Name::new("Square"),
    ));

    // TODO: Add instancing later using ECS patterns

    info!("Initial ECS entities spawned.");
}


/// Update system: Handles window resize events. (Simplified)
fn handle_resize_system(
    mut resize_reader: EventReader<bevy_window::WindowResized>,
    renderer_res_opt: Option<ResMut<RendererResource>>, // Use ResMut
    vk_context_res_opt: Option<Res<VulkanContextResource>>,
) {
    let Some(renderer_res) = renderer_res_opt else { return; };
    let Some(vk_context_res) = vk_context_res_opt else { return; };

    for event in resize_reader.read() {
        info!("WindowResized event: {:?}", event);
        if event.width > 0.0 && event.height > 0.0 {
            if let (Ok(mut renderer_guard), Ok(mut vk_ctx_guard)) = ( // vk_ctx needs mut lock too
                renderer_res.0.lock(),
                vk_context_res.0.lock(),
            ) {
                info!("Calling actual resize logic.");
                // Pass mutable references from the guards
                renderer_guard.resize_renderer(&mut vk_ctx_guard, event.width as u32, event.height as u32);
            } else {
                warn!("Could not lock resources for resize handling.");
            }
        }
    }
}

/// Update system: Processes mouse input for clicking and dragging. (Modified for Arc<Vec<Vertex>>)
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


/// Update system: Applies movement deltas from drag events to Transforms.
fn movement_system(
    mut drag_evr: EventReader<EntityDragged>,
    mut query: Query<&mut Transform>,
) {
    for ev in drag_evr.read() {
        if let Ok(mut transform) = query.get_mut(ev.entity) {
            transform.translation.x += ev.delta.x;
            // OLD: transform.translation.y -= ev.delta.y; // Invert Y delta
            // NEW: Apply Y delta directly
            transform.translation.y += ev.delta.y;
        }
    }
}


/// Update system: Detects keyboard input and sends HotkeyActionTriggered events. (No changes needed)
fn hotkey_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    hotkey_config: Res<HotkeyResource>,
    mut hotkey_evw: EventWriter<HotkeyActionTriggered>,
) {
    // (Hotkey detection logic remains the same)
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

/// Update system: Handles application control actions (e.g., exit). (No changes needed)
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


/// Update system: Triggers rendering via the custom Vulkan Renderer. (Modified)
fn rendering_system(
    renderer_res_opt: Option<ResMut<RendererResource>>, // Use ResMut
    vk_context_res_opt: Option<Res<VulkanContextResource>>,
    // Query for renderable entities using ECS components
    // Add GlobalTransform to get the final world matrix easily
    query: Query<(Entity, &GlobalTransform, &ShapeData, &Visibility)>,
) {
    if let (Some(renderer_res), Some(vk_context_res)) =
        (renderer_res_opt, vk_context_res_opt)
    {
        // --- Collect Render Data from ECS ---
        let mut render_commands: Vec<RenderCommandData> = Vec::new();
        for (entity, global_transform, shape, visibility) in query.iter() {
            if visibility.0 { // Check custom visibility component
                // Populate RenderCommandData from components
                render_commands.push(RenderCommandData {
                    entity_id: entity,
                    // Get the computed world matrix from GlobalTransform
                    transform_matrix: global_transform.compute_matrix(),
                    vertices: shape.vertices.clone(), // Clone the Arc
                    vertex_shader_path: shape.vertex_shader_path.clone(),
                    fragment_shader_path: shape.fragment_shader_path.clone(),
                    // Use Z translation for depth sorting
                    depth: global_transform.translation().z,
                });
            }
        }

        // Sort render_commands by depth (higher Z drawn later/on top)
        render_commands.sort_unstable_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap_or(std::cmp::Ordering::Equal));

        // --- Call Custom Renderer ---
        if let (Ok(mut renderer_guard), Ok(mut vk_ctx_guard)) = ( // vk_ctx needs mut lock too
            renderer_res.0.lock(),
            vk_context_res.0.lock(),
        ) {
            // Pass collected and sorted data to the actual renderer's render method
            renderer_guard.render(&mut vk_ctx_guard, &render_commands); // Pass &mut guard and commands
        } else {
            warn!("Could not lock resources for rendering trigger.");
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

/// System running on AppExit in Update schedule: Takes ownership of Vulkan/Renderer resources via World access and cleans them up immediately.
fn cleanup_trigger_system(world: &mut World) {
    info!("ENTERED cleanup_trigger_system (on AppExit)");

    // --- Take Ownership of Resources ---
    // Remove the resources from the World immediately using direct World access.
    // This returns Option<T>, giving us ownership.
    let renderer_res_opt: Option<RendererResource> = world.remove_resource::<RendererResource>();
    let vk_context_res_opt: Option<VulkanContextResource> = world.remove_resource::<VulkanContextResource>();

    // --- Perform Cleanup ---
    if let Some(vk_context_res) = vk_context_res_opt {
        info!("VulkanContextResource taken.");
        match vk_context_res.0.lock() {
            Ok(mut vk_ctx_guard) => {
                info!("Successfully locked VulkanContext Mutex.");

                // 1. Cleanup Renderer (if it existed)
                if let Some(renderer_res) = renderer_res_opt {
                    info!("RendererResource taken.");
                    match renderer_res.0.lock() {
                        Ok(mut renderer_guard) => {
                            info!("Successfully locked Renderer Mutex.");
                            info!("Calling actual Renderer cleanup via MutexGuard.");
                            renderer_guard.cleanup(&mut vk_ctx_guard); // Pass vk_ctx guard
                        }
                        Err(poisoned) => {
                            error!("Renderer Mutex was poisoned before cleanup: {:?}", poisoned);
                        }
                    }
                } else {
                    info!("Renderer resource not found or already removed.");
                }

                // 2. Cleanup Vulkan Context
                info!("Calling cleanup_vulkan...");
                cleanup_vulkan(&mut vk_ctx_guard); // Pass vk_ctx guard
                info!("cleanup_vulkan finished.");

            } // vk_ctx_guard dropped here
            Err(poisoned) => {
                error!("VulkanContext Mutex was poisoned before cleanup: {:?}", poisoned);
            }
        }
    } else {
        warn!("VulkanContext resource not found or already removed during cleanup trigger.");
    }

    // --- Resources go out of scope ---
    info!("Cleanup trigger system finished, taken resources going out of scope.");
    // Drop implementations (including Allocator) run here *after* cleanup_vulkan.
}