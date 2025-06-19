use bevy_app::{App, Plugin, Startup, Update};
use bevy_ecs::prelude::*;
use bevy_window::{PrimaryWindow, Window, WindowResized, WindowPlugin, WindowResolution, PresentMode};
use bevy_log::{info, warn, error};
use bevy_core::{Name, TaskPoolPlugin};
use bevy_color::Color;
use bevy_transform::{prelude::Transform, TransformPlugin};
use bevy_input::InputPlugin;
use bevy_winit::{WinitPlugin, WakeUp};
use bevy_a11y::AccessibilityPlugin;
use bevy_hierarchy::HierarchyPlugin;
use bevy_asset::AssetPlugin;
use bevy_utils::default;
use std::sync::{Arc, Mutex};
use yrs::Doc;

use crate::{
    VulkanContext,
    VulkanContextResource,
    YrsDocResource,
    gui_framework::plugins::{
        core::GuiFrameworkCorePlugin,
        interaction::GuiFrameworkInteractionPlugin,
        movement::GuiFrameworkDefaultMovementPlugin,
        bindings::GuiFrameworkDefaultBindingsPlugin,
    },
    layout::TaffyLayoutPlugin,
    assets::{UiAssetPlugin, LoadUiRequest},
    ShapeData,
    Vertex,
    Visibility,
    Interaction,
};

#[derive(Component)]
struct BackgroundQuad;

pub struct WhipUiPlugin {
    root_layout_path: String,
}

impl WhipUiPlugin {
    pub fn new(root_layout_path: &str) -> Self {
        Self {
            root_layout_path: root_layout_path.to_string(),
        }
    }
}

impl Plugin for WhipUiPlugin {
    fn build(&self, app: &mut App) {
        info!("Initializing WhipUiPlugin with layout: {}", self.root_layout_path);

        // Add all essential Bevy plugins that the framework requires
        app.add_plugins((
            TaskPoolPlugin::default(),
            bevy_time::TimePlugin::default(),
            TransformPlugin::default(),
            InputPlugin::default(),
            WindowPlugin {
                primary_window: Some(Window {
                    title: "WhipUI Application".into(),
                    resolution: WindowResolution::new(600.0, 300.0), // Default, will be updated from TOML
                    present_mode: PresentMode::AutoVsync,
                    ..default()
                }),
                ..default()
            },
            AccessibilityPlugin,
            WinitPlugin::<WakeUp>::default(),
            HierarchyPlugin::default(),
            AssetPlugin::default(),
        ));

        // Initialize framework resources
        let vulkan_context = Arc::new(Mutex::new(VulkanContext::new()));
        app.insert_resource(VulkanContextResource(vulkan_context))
           .insert_resource(YrsDocResource {
               doc: Arc::new(Doc::new()),
               text_map: Arc::new(Mutex::new(std::collections::HashMap::new())),
           });

        // Add all framework plugins in correct order
        app.add_plugins(GuiFrameworkCorePlugin)
           .add_plugins(GuiFrameworkInteractionPlugin)
           .add_plugins(GuiFrameworkDefaultMovementPlugin)
           .add_plugins(GuiFrameworkDefaultBindingsPlugin)
           .add_plugins(UiAssetPlugin)
           .add_plugins(TaffyLayoutPlugin);

        // Store the layout path for startup system
        app.insert_resource(RootLayoutPath(self.root_layout_path.clone()));

        // Add systems
        app.add_systems(Startup, (setup_initial_scene, load_root_layout))
           .add_systems(Update, (apply_window_config_from_asset, background_resize_system));

        info!("WhipUiPlugin initialized successfully");
    }
}

#[derive(Resource)]
struct RootLayoutPath(String);

fn setup_initial_scene(
    mut commands: Commands,
    primary_window_q: Query<&Window, With<PrimaryWindow>>,
) {
    info!("Setting up initial scene...");
    
    let Ok(primary_window) = primary_window_q.get_single() else {
        error!("Primary window not found in setup_initial_scene!");
        return;
    };
    
    let logical_width = primary_window.width();
    let logical_height = primary_window.height();
    
    info!("Using logical dimensions for background: {}x{}", logical_width, logical_height);

    // Default background color (will be updated when window config loads)
    let background_color = Color::srgba(0.129, 0.161, 0.165, 1.0);

    // Spawn background quad
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
}

fn load_root_layout(
    mut load_ui_events: EventWriter<LoadUiRequest>,
    root_layout_path: Res<RootLayoutPath>,
) {
    info!("Loading root layout: {}", root_layout_path.0);
    load_ui_events.send(LoadUiRequest::new(&root_layout_path.0));
}

fn apply_window_config_from_asset(
    mut window_config_events: EventReader<bevy_asset::AssetEvent<crate::assets::UiTree>>,
    ui_trees: Res<bevy_asset::Assets<crate::assets::UiTree>>,
    mut primary_window_q: Query<&mut Window, With<PrimaryWindow>>,
    mut background_query: Query<&mut ShapeData, With<BackgroundQuad>>,
    mut commands: Commands,
) {
    for event in window_config_events.read() {
        if let bevy_asset::AssetEvent::LoadedWithDependencies { id } = event {
            if let Some(ui_tree) = ui_trees.get(*id) {
                let window_config = &ui_tree.window;
                info!("Applying window config from asset");
                
                // Insert window config as resource
                commands.insert_resource(window_config.clone());
                
                // Apply window size
                if let Ok(mut window) = primary_window_q.get_single_mut() {
                    window.resolution.set(window_config.size[0], window_config.size[1]);
                    info!("Updated window size to: {}x{}", window_config.size[0], window_config.size[1]);
                }
                
                // Update background color if specified
                if let Some(bg_color) = window_config.background_color {
                    if let Ok(mut shape_data) = background_query.get_single_mut() {
                        shape_data.color = bg_color;
                        info!("Updated background color");
                    }
                }
            }
        }
    }
}

fn background_resize_system(
    mut resize_reader: EventReader<WindowResized>,
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