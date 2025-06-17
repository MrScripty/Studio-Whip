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
    GuiFrameworkCorePlugin,
    GuiFrameworkInteractionPlugin,
    GuiFrameworkDefaultMovementPlugin,
    GuiFrameworkDefaultBindingsPlugin,
    UiAssetPlugin,
    LoadUiRequest,
    TaffyLayoutPlugin,
    WindowConfig,
};

// Import Bevy Name for debugging entities
use bevy_core::Name;
use bevy_color::Color;
// Import Yrs types needed for resource initialization
use yrs::Doc;
use std::sync::{Arc, Mutex};

#[derive(Component)]
struct BackgroundQuad;

/// Load window configuration from TOML file synchronously
fn load_window_config() -> WindowConfig {
    let toml_path = "assets/ui/layouts/main.toml";
    
    match std::fs::read_to_string(toml_path) {
        Ok(toml_content) => {
            match toml::from_str::<toml::Value>(&toml_content) {
                Ok(parsed_toml) => {
                    parse_window_config_direct(&parsed_toml).unwrap_or_else(|e| {
                        bevy_log::warn!("Failed to parse window config: {}, using defaults", e);
                        WindowConfig::default()
                    })
                }
                Err(e) => {
                    bevy_log::warn!("Failed to parse TOML: {}, using default window config", e);
                    WindowConfig::default()
                }
            }
        }
        Err(e) => {
            bevy_log::warn!("Failed to read TOML file {}: {}, using default window config", toml_path, e);
            WindowConfig::default()
        }
    }
}

/// Parse window configuration directly from TOML value
fn parse_window_config_direct(toml_value: &toml::Value) -> Result<WindowConfig, String> {
    let table = toml_value.as_table()
        .ok_or_else(|| "Root must be a table".to_string())?;
    
    let mut window_config = WindowConfig::default();
    
    if let Some(window_section) = table.get("window") {
        if let Some(window_table) = window_section.as_table() {
            // Parse window size
            if let Some(size_array) = window_table.get("size") {
                if let Some(size_arr) = size_array.as_array() {
                    if size_arr.len() == 2 {
                        if let (Some(width), Some(height)) = (size_arr[0].as_float(), size_arr[1].as_float()) {
                            window_config.size = [width as f32, height as f32];
                        }
                    }
                }
            }
            
            // Parse background color
            if let Some(bg_color) = window_table.get("background_color") {
                if let Some(color_table) = bg_color.as_table() {
                    if let (Some(r), Some(g), Some(b), Some(a)) = (
                        color_table.get("r").and_then(|v| v.as_integer()),
                        color_table.get("g").and_then(|v| v.as_integer()),
                        color_table.get("b").and_then(|v| v.as_integer()),
                        color_table.get("a").and_then(|v| v.as_float()),
                    ) {
                        window_config.background_color = Some(Color::srgba(
                            r as f32 / 255.0,
                            g as f32 / 255.0,
                            b as f32 / 255.0,
                            a as f32,
                        ));
                    }
                }
            }
        }
    }
    
    Ok(window_config)
}

fn main() {
    info!("Starting whip_ui example...");

    // Load window configuration from TOML
    let window_config = load_window_config();
    info!("Loaded window config: {}x{}", window_config.size[0], window_config.size[1]);

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
                    resolution: WindowResolution::new(window_config.size[0], window_config.size[1]),
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
        .insert_resource(window_config.clone())
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
    _yrs_res: ResMut<YrsDocResource>,
    window_config: Res<WindowConfig>,
) {
    info!("Running setup_scene_ecs...");
    
    let Ok(primary_window) = primary_window_q.get_single() else {
        error!("Primary window not found in setup_scene_ecs!");
        return;
   };
   let logical_width = primary_window.width();
   let logical_height = primary_window.height();

   info!("Using logical dimensions for background: {}x{}", logical_width, logical_height);

    // Background color from window config
    let background_color = window_config.background_color.unwrap_or(Color::srgba(0.129, 0.161, 0.165, 1.0));

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
        ], background_color),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Visibility(true),
        Interaction::default(),
        Name::new("Background"),
        BackgroundQuad,
    ));

    // All UI widgets will now be loaded from TOML via the asset loading system
    // The test_asset_loading system will trigger the LoadUiRequest event
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