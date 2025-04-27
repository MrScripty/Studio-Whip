use bevy_app::{App, Startup, Update};
use bevy_ecs::prelude::*;
use bevy_log::{info, error, warn, LogPlugin, Level};
use bevy_utils::default;
use bevy_input::InputPlugin;
use bevy_window::{
    PrimaryWindow, Window, WindowPlugin, PresentMode,
    WindowResolution,
};
use bevy_winit::{WinitPlugin, WakeUp};
use bevy_a11y::AccessibilityPlugin;
use bevy_transform::prelude::Transform; // Keep for setup_scene_ecs
use bevy_transform::TransformPlugin;
// Import types defined in lib.rs
use rusty_whip::{Vertex, VulkanContextResource};
use std::sync::{Arc, Mutex};
// Import framework components, events, resources, and the new plugin
use rusty_whip::gui_framework::{
    VulkanContext,
    components::{ShapeData, Visibility, Interaction, Text, TextAlignment},
    plugins::core::GuiFrameworkCorePlugin,
    // Import plugin SystemSets for ordering
    plugins::core::CoreSet,
    plugins::interaction::GuiFrameworkInteractionPlugin,
    plugins::movement::GuiFrameworkDefaultMovementPlugin,
    plugins::bindings::GuiFrameworkDefaultBindingsPlugin,
};
// Import Bevy Name for debugging entities
use bevy_core::Name;
use bevy_color::Color;


#[derive(Component)]
struct BackgroundQuad;

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
        // == Add Framework Plugins ==
        .add_plugins(GuiFrameworkCorePlugin)
        .add_plugins(GuiFrameworkInteractionPlugin) 
        .add_plugins(GuiFrameworkDefaultMovementPlugin) 
        .add_plugins(GuiFrameworkDefaultBindingsPlugin) 

        // == Startup Systems ==
        .add_systems(Startup,
            // Ensure app scene setup runs after core renderer and hotkey loading are done
            setup_scene_ecs
                .after(CoreSet::CreateRenderer) // From core plugin
                // .after(InteractionSet::LoadHotkeys) // Optional: If setup needs hotkeys loaded
        )
        // == Update Systems ==
        .add_systems(Update,
            (
                // App specific systems
                background_resize_system,
            )
        )
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
            vertices: Arc::new(vec![
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
                Vertex { position: [-25.0, -25.0] },
                Vertex { position: [-25.0, 25.0] },
                Vertex { position: [25.0, -25.0] },
                // Triangle 2
                Vertex { position: [25.0, -25.0] },
                Vertex { position: [-25.0, 25.0] },
                Vertex { position: [25.0, 25.0] },
            ]),
            vertex_shader_path: "square.vert.spv".to_string(),
            fragment_shader_path: "square.frag.spv".to_string(),
        },
        Transform::from_xyz(125.0, 75.0, 2.0),
        Visibility(true),
        Interaction { clickable: true, draggable: true },
        Name::new("Square"),
    ));

    // --- Spawn Sample Text ---
    commands.spawn((
        Text {
            content: "Hello, Rusty Whip!\nThis is a test of text rendering.".to_string(),
            size: 24.0, // Font size
            color: Color::WHITE, // Text color
            alignment: TextAlignment::Left, // Text alignment
            bounds: None, // No specific bounds for wrapping initially
        },
        Transform::from_xyz(50.0, 250.0, 3.0), // Position the text (ensure Z > shapes if needed)
        Visibility(true),
        Name::new("SampleText"),
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