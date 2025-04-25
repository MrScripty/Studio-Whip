use bevy_app::{App, AppExit, Startup, Update};
use bevy_ecs::prelude::*;
//use bevy_ecs::change_detection::DetectChanges;
use bevy_log::{info, error, warn, LogPlugin, Level};
use bevy_utils::default;
use bevy_input::InputPlugin; // Keep for app_control_system
use bevy_window::{
    PrimaryWindow, Window, WindowPlugin, PresentMode,
    WindowResolution, // Keep for background_resize_system
};
use bevy_winit::{WinitPlugin, WakeUp};
use bevy_a11y::AccessibilityPlugin;
use bevy_transform::prelude::Transform; // Keep for movement_system & setup_scene_ecs
use bevy_transform::TransformPlugin;
// Import types defined in lib.rs
use rusty_whip::{Vertex, VulkanContextResource}; // Import resources from lib.rs

use std::sync::{Arc, Mutex};
// Import framework components, events, resources, and the new plugin
use rusty_whip::gui_framework::{
    VulkanContext, // Still needed for Resource definition in lib.rs and initial creation
    components::{ShapeData, Visibility, Interaction}, // Still needed for setup_scene_ecs
    events::{EntityDragged, HotkeyActionTriggered}, // Still needed for event handling
    plugins::core::GuiFrameworkCorePlugin, // <-- Import the core plugin
    plugins::interaction::GuiFrameworkInteractionPlugin, // <-- Import the interaction plugin
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
// #[derive(Resource, Debug, Clone, Default, Reflect)]
// struct HotkeyResource(HotkeyConfig); // Defined in lib.rs now

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
        // HotkeyResource inserted by GuiFrameworkInteractionPlugin's load_hotkeys_system
        // RendererResource inserted by GuiFrameworkCorePlugin's create_renderer_system

        // == Events ==
        // Events are added by GuiFrameworkInteractionPlugin

        // == Reflection Registration ==
        // Core types (Vertex, ShapeData, Visibility) registered by GuiFrameworkCorePlugin
        // Interaction types/events registered by GuiFrameworkInteractionPlugin

        // == Add Framework Plugins ==
        .add_plugins(GuiFrameworkCorePlugin) // <-- Add the core plugin
        .add_plugins(GuiFrameworkInteractionPlugin) // <-- Add the interaction plugin

        // == Startup Systems ==
        .add_systems(Startup,
            (
                setup_scene_ecs, // Setup app-specific ECS scene (runs after plugin setup)
            )//.after(...) // TODO: Use System Sets later for explicit ordering if needed
        )
        // == Update Systems ==
        .add_systems(Update,
            (
                // App specific systems
                background_resize_system, // App specific background handling
                movement_system,          // App specific movement logic
                app_control_system,       // App specific hotkey->action mapping (exit)
            )
        )
        // == Rendering System (runs late) ==
        // Moved to GuiFrameworkCorePlugin

        // == Shutdown System ==
        // Moved to GuiFrameworkCorePlugin

        // == Run the App ==
        .run();
}

// --- Bevy Systems ---

/// Startup system: Spawns initial ECS entities for the application.
fn setup_scene_ecs(
    mut commands: Commands,
    primary_window_q: Query<&Window, With<PrimaryWindow>>,
) {
    info!("Running setup_scene_ecs...");

    // --- Hotkey Configuration Loading Moved to GuiFrameworkInteractionPlugin ---

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
        Interaction { clickable: true, draggable: true }, // Handled by InteractionPlugin
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
        Interaction { clickable: true, draggable: true }, // Handled by InteractionPlugin
        Name::new("Square"),
    ));

    // TODO: Add instancing later using ECS patterns

    info!("Initial ECS entities spawned.");
}

// System to update background vertices on resize (App specific)
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

// Update system: Applies movement deltas from drag events to Transforms (App specific)
fn movement_system(
    mut drag_evr: EventReader<EntityDragged>, // Reads events from InteractionPlugin
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

// Update system: Handles application control actions (e.g., exit) based on hotkeys (App specific)
fn app_control_system(
    mut hotkey_evr: EventReader<HotkeyActionTriggered>, // Reads events from InteractionPlugin
    mut app_exit_evw: EventWriter<AppExit>,
) {
    for ev in hotkey_evr.read() {
        // This system decides what specific actions mean
        if ev.action == "CloseRequested" {
            info!("'CloseRequested' hotkey action received, sending AppExit.");
            app_exit_evw.send(AppExit::Success);
        }
        // Add other hotkey action handling here if needed
    }
}

// Systems moved to GuiFrameworkCorePlugin:
// - setup_vulkan_system
// - create_renderer_system
// - handle_resize_system
// - rendering_system
// - cleanup_trigger_system

// Systems moved to GuiFrameworkInteractionPlugin:
// - interaction_system
// - hotkey_system
// - handle_close_request
// - load_hotkeys_system (extracted from setup_scene_ecs)