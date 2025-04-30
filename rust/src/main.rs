use bevy_app::{App, Startup, Update};
use bevy_ecs::prelude::*;
use bevy_log::{info, error, warn, LogPlugin, Level};
use bevy_utils::{default, HashMap};
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
    plugins::interaction::InteractionSet,
    plugins::movement::GuiFrameworkDefaultMovementPlugin,
    plugins::bindings::GuiFrameworkDefaultBindingsPlugin,
};
// Import Bevy Name for debugging entities
use bevy_core::Name;
use bevy_color::Color;
// Import Yrs types needed for resource initialization
use yrs::{Doc, Text as YrsText}; // Alias YrsText to avoid conflict
use rusty_whip::YrsDocResource; // Import the new resource type
use yrs::TextRef;
use yrs::Transact;

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
        .insert_resource(VulkanContextResource(vulkan_context))
        .insert_resource(YrsDocResource {
            doc: Doc::new(),
            text_map: std::collections::HashMap::new(),
        })
        // == Add Framework Plugins ==
        .add_plugins(GuiFrameworkCorePlugin)
        .add_plugins(GuiFrameworkInteractionPlugin) 
        .add_plugins(GuiFrameworkDefaultMovementPlugin) 
        .add_plugins(GuiFrameworkDefaultBindingsPlugin) 

        // == Startup Systems ==
        .add_systems(Startup,
            // Ensure app scene setup runs after all core resources are created
            setup_scene_ecs
                .after(CoreSet::CreateTextResources) // Depends on the last core resource creation set
                .after(InteractionSet::LoadHotkeys) // Also ensure hotkeys are loaded if needed by scene setup
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
    mut yrs_res: ResMut<YrsDocResource>,
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
        Transform::from_xyz(300.0, 150.0, -1.0),
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
        Transform::from_xyz(125.0, 75.0, -2.0),
        Visibility(true),
        Interaction { clickable: true, draggable: true },
        Name::new("Square"),
    ));

    // --- Spawn Sample Text ---
    // 1. Create the YrsText data within the YrsDoc using the system parameter
    let yrs_text_content = "Hello, Studio Whip!\nThis is collaborative text.".to_string();
    // Access yrs_res directly (it's already mutable)
    let text_handle: yrs::TextRef = { // Explicit type annotation can help clarity
        let text_ref = yrs_res.doc.get_or_insert_text("sample_text"); // Use a unique name for the YrsText
        let mut txn = yrs_res.doc.transact_mut();
        text_ref.insert(&mut txn, 0, &yrs_text_content);
        text_ref // Return the handle
    }; // yrs_res mutable borrow ends here if txn is dropped

    // 2. Spawn the Bevy entity and store the mapping
    let text_entity = commands.spawn((
        // Text component no longer has 'content'
        Text {
            size: 24.0,
            color: Color::WHITE,
            alignment: TextAlignment::Left,
            bounds: None,
        },
        Transform::from_xyz(50.0, 250.0, -2.0), // Position the text (ensure Z > shapes if needed)
        Visibility(true),
        Name::new("SampleText"),
    )).id(); // Get the entity ID after spawning

    // 3. Update the YrsDocResource map using the system parameter
    // No need for Option<> here, ResMut will panic if resource doesn't exist
    yrs_res.text_map.insert(text_entity, text_handle);
    info!("Mapped Entity {:?} to YrsText 'sample_text'", text_entity);


    // TODO: Add instancing later using ECS patterns
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