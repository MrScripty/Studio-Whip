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
use bevy_transform::prelude::Transform;
use bevy_transform::TransformPlugin;
use bevy_hierarchy::HierarchyPlugin;
use bevy_asset::AssetPlugin;
use bevy_tasks::IoTaskPool;
use bevy_core::TaskPoolPlugin;

// Import from whip_ui library
use whip_ui::{
    Vertex, 
    VulkanContextResource,
    YrsDocResource,
    VulkanContext,
    ShapeData, 
    Visibility, 
    Interaction, 
    Text, 
    TextAlignment, 
    EditableText,
    GuiFrameworkCorePlugin,
    GuiFrameworkInteractionPlugin,
    GuiFrameworkDefaultMovementPlugin,
    GuiFrameworkDefaultBindingsPlugin,
    UiAssetPlugin,
    LoadUiRequest,
    TaffyLayoutPlugin,
    UiNode,
    Styleable,
    PositionControl,
};

// Import Bevy Name for debugging entities
use bevy_core::Name;
use bevy_color::Color;
// Import Yrs types needed for resource initialization
use yrs::{Doc, Text as YrsText}; // Alias YrsText to avoid conflict
use yrs::Transact;
use std::sync::{Arc, Mutex};
// Import Taffy for layout styles
use taffy;

#[derive(Component)]
struct BackgroundQuad;

fn main() {
    info!("Starting whip_ui example...");

    // Initialize IoTaskPool manually
    IoTaskPool::get_or_init(|| {
        bevy_tasks::TaskPool::new()
    });

    // --- Initialize Vulkan Context ---
    let vulkan_context = Arc::new(Mutex::new(VulkanContext::new()));

    // --- Build Bevy App ---
    App::new()
        .add_plugins((
            LogPlugin { level: Level::DEBUG, filter: "wgpu=error,naga=warn,bevy_app=info,bevy_ecs=info,whip_ui=debug".to_string(), ..default() },
            TaskPoolPlugin::default(),
            bevy_time::TimePlugin::default(),
            TransformPlugin::default(),
            InputPlugin::default(),
            WindowPlugin {
                primary_window: Some(Window {
                    title: "whip_ui Example".into(),
                    resolution: WindowResolution::new(600.0, 300.0),
                    present_mode: PresentMode::AutoVsync,
                    ..default()
                }),
                ..default()
            },
            AccessibilityPlugin,
            WinitPlugin::<WakeUp>::default(),
            HierarchyPlugin::default(),
            AssetPlugin::default(),
        ))
        // == Resources ==
        .insert_resource(VulkanContextResource(vulkan_context))
        .insert_resource(YrsDocResource {
            doc: Arc::new(Doc::new()),
            text_map: Arc::new(Mutex::new(std::collections::HashMap::new())),
        })
        // == Add Framework Plugins ==
        .add_plugins(GuiFrameworkCorePlugin)
        .add_plugins(GuiFrameworkInteractionPlugin)
        .add_plugins(GuiFrameworkDefaultMovementPlugin)
        .add_plugins(GuiFrameworkDefaultBindingsPlugin)
        .add_plugins(UiAssetPlugin)
        .add_plugins(TaffyLayoutPlugin)

        // == Startup Systems ==
        .add_systems(Startup, (setup_scene_ecs, test_asset_loading))
        // == Update Systems ==
        .add_systems(Update, background_resize_system)
        // == Run the App ==
        .run();
}

/// Startup system: Spawns initial ECS entities for the application.
fn setup_scene_ecs(
    mut commands: Commands,
    primary_window_q: Query<&Window, With<PrimaryWindow>>,
    yrs_res: ResMut<YrsDocResource>,
) {
    info!("Running setup_scene_ecs...");
    
    let Ok(primary_window) = primary_window_q.get_single() else {
        error!("Primary window not found in setup_scene_ecs!");
        return;
   };
   let logical_width = primary_window.width();
   let logical_height = primary_window.height();

   info!("Using logical dimensions for background: {}x{}", logical_width, logical_height);

    // Background (Not interactive, covers full screen) - Use custom vertices for exact fit
    commands.spawn((
        ShapeData::new(vec![
            // Triangle 1
            Vertex { position: [0.0, 0.0] },
            Vertex { position: [0.0, logical_height] },
            Vertex { position: [logical_width, 0.0] },
            // Triangle 2
            Vertex { position: [logical_width, 0.0] },
            Vertex { position: [0.0, logical_height] },
            Vertex { position: [logical_width, logical_height] },
        ], Color::srgba(0.129, 0.161, 0.165, 1.0)),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Visibility(true),
        Interaction::default(),
        Name::new("Background"),
        BackgroundQuad,
    ));

    // Triangle (Draggable and Clickable) - Orange - Manual positioning  
    commands.spawn((
        ShapeData::triangle(50.0, 50.0, Color::srgba(1.0, 0.596, 0.0, 1.0)),
        Transform::from_xyz(300.0, 150.0, -1.0),
        Visibility(true),
        Interaction { clickable: true, draggable: true },
        PositionControl::Manual, // Manual positioning for draggable shapes
        Name::new("Triangle"),
    ));

    // Square (Draggable and Clickable) - Green - Manual positioning
    commands.spawn((
        ShapeData::rectangle(50.0, 50.0, Color::srgba(0.259, 0.788, 0.133, 1.0)),
        Transform::from_xyz(125.0, 75.0, -2.0),
        Visibility(true),
        Interaction { clickable: true, draggable: true },
        PositionControl::Manual, // Manual positioning for draggable shapes
        Name::new("Square"),
    ));

    // --- Spawn Sample Text ---
    let yrs_text_content = "Hello, whip_ui!\nThis is collaborative text.".to_string();
    let text_handle: yrs::TextRef = {
        let text_ref = yrs_res.doc.get_or_insert_text("sample_text");
        let mut txn = yrs_res.doc.transact_mut();
        text_ref.insert(&mut txn, 0, &yrs_text_content);
        text_ref
    };

    let text_entity = commands.spawn((
        Text {
            size: 24.0,
            color: Color::BLACK,
            alignment: TextAlignment::Left,
            bounds: None,
        },
        Transform::from_xyz(50.0, 250.0, -2.0),
        Visibility(true),
        Name::new("SampleText"),
        EditableText,
        PositionControl::Layout, // Use layout positioning for text
        UiNode::default(),
        Styleable(taffy::Style {
            position: taffy::Position::Absolute,
            inset: taffy::Rect {
                left: taffy::LengthPercentageAuto::Length(50.0),
                top: taffy::LengthPercentageAuto::Length(50.0), // 50px from top of window = Y=250 in Bevy
                right: taffy::LengthPercentageAuto::Auto,
                bottom: taffy::LengthPercentageAuto::Auto,
            },
            size: taffy::Size {
                width: taffy::Dimension::Length(200.0), // Explicit width for text
                height: taffy::Dimension::Length(48.0), // Explicit height for text  
            },
            ..Default::default()
        }),
    )).id();

    yrs_res.text_map.lock().expect("Failed to lock text_map mutex in setup").insert(text_entity, text_handle);
    info!("Mapped Entity {:?} to YrsText 'sample_text'", text_entity);
}

// System to update background vertices on resize (App specific)
fn background_resize_system(
    mut resize_reader: EventReader<bevy_window::WindowResized>,
    mut background_query: Query<&mut ShapeData, With<BackgroundQuad>>,
) {
    for event in resize_reader.read() {
        if event.width > 0.0 && event.height > 0.0 {
            let logical_width = event.width;
            let logical_height = event.height;

            if let Ok(mut shape_data) = background_query.get_single_mut() {
                shape_data.vertices = Arc::new(vec![
                    Vertex { position: [0.0, 0.0] },
                    Vertex { position: [0.0, logical_height] },
                    Vertex { position: [logical_width, 0.0] },
                    Vertex { position: [logical_width, 0.0] },
                    Vertex { position: [0.0, logical_height] },
                    Vertex { position: [logical_width, logical_height] },
                ]);
            } else {
                warn!("BackgroundQuad entity ShapeData not found during resize update.");
            }
        }
    }
}

/// Test system to load UI assets and verify Milestone 2 functionality
fn test_asset_loading(
    mut load_ui_events: EventWriter<LoadUiRequest>,
) {
    info!("Testing asset loading - sending LoadUiRequest for ui/layouts/main.toml");
    
    // Test loading the main layout file
    load_ui_events.send(LoadUiRequest::new("ui/layouts/main.toml"));
    
    info!("LoadUiRequest sent - asset loading system should process it");
}