use bevy_app::{App, AppExit, Plugin, Startup, Update, Last};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::common_conditions::{not, on_event};
use bevy_log::{info, error, warn};
use bevy_window::{PrimaryWindow, Window, WindowResized};
use bevy_winit::WinitWindows;
use bevy_transform::prelude::GlobalTransform;
use bevy_reflect::Reflect;
use std::sync::{Arc, Mutex};
use std::collections::HashSet; // For rendering_system change detection
use ash::vk;

// Import types from the crate root (lib.rs)
use crate::{Vertex, RenderCommandData};

// Import types/functions from the gui_framework
use crate::gui_framework::{
    VulkanContext,
    context::vulkan_setup::{setup_vulkan, cleanup_vulkan},
    rendering::render_engine::Renderer,
    components::{ShapeData, Visibility},
};

// Import resources used/managed by this plugin's systems
// These resources are defined in lib.rs and inserted elsewhere (main.rs or this plugin)
use crate::{VulkanContextResource, RendererResource};

// --- Core Plugin Definition ---

pub struct GuiFrameworkCorePlugin;

impl Plugin for GuiFrameworkCorePlugin {
    fn build(&self, app: &mut App) {
        info!("Building GuiFrameworkCorePlugin...");

        // --- Type Registration ---
        // Register components used/managed by the core rendering systems
        app.register_type::<ShapeData>();
        app.register_type::<Visibility>();
        // Register Vertex type from lib.rs (ensure it derives Reflect + TypePath)
        app.register_type::<Vertex>();
        // Note: Resources like VulkanContextResource and RendererResource
        // usually don't need explicit registration if they derive Resource.
        // VulkanContextResource is inserted in main.rs.
        // RendererResource is inserted by create_renderer_system below.

        // --- System Setup ---
        app
            // == Startup Systems ==
            // Setup Vulkan -> Create Renderer
            // RendererResource is inserted here
            // Pass the piped system configuration directly
            .add_systems(Startup, setup_vulkan_system.pipe(create_renderer_system))
            // == Update Systems ==
            .add_systems(Update,(
                // Vulkan Renderer specific updates
                handle_resize_system,
                )
            )
            // == Rendering System (runs late) ==
            // Run rendering system only if AppExit hasn't been sent this frame
            .add_systems(Last, (
                // Run rendering system only if AppExit hasn't been sent this frame
                rendering_system.run_if(not(on_event::<AppExit>)),
                // Run cleanup system *if* AppExit has been sent
                cleanup_trigger_system.run_if(on_event::<AppExit>),
            )); // Systems in Last run concurrently by default unless chained

        info!("GuiFrameworkCorePlugin built.");
    }
}


// --- Systems Moved from main.rs ---

/// Startup system: Initializes Vulkan using the primary window handle.
fn setup_vulkan_system(
    vk_context_res: Res<VulkanContextResource>,
    primary_window_q: Query<Entity, With<PrimaryWindow>>,
    winit_windows: NonSend<WinitWindows>,
) { // Changed return type to ()
    info!("Running setup_vulkan_system (Core Plugin)...");
    let primary_entity = primary_window_q.get_single()
        .expect("Failed to get primary window entity"); // Panic on error
    let winit_window = winit_windows.get_window(primary_entity)
        .expect("Failed to get winit window reference from WinitWindows"); // Panic on error

    // Use expect for mutex lock during startup
    let mut vk_ctx_guard = vk_context_res.0.lock().expect("Failed to lock VulkanContext mutex for setup");

    setup_vulkan(&mut vk_ctx_guard, winit_window);
    info!("Vulkan setup complete (Core Plugin).");
    // No return value needed
}

/// Startup system (piped): Creates the Renderer instance resource.
fn create_renderer_system(
    // No longer needs In<> because previous system panics on error
    mut commands: Commands,
    vk_context_res: Res<VulkanContextResource>,
    primary_window_q: Query<&Window, With<PrimaryWindow>>,
) { // Changed return type to ()
    info!("Running create_renderer_system (Core Plugin)...");

    let primary_window = primary_window_q.get_single().expect("Primary window not found");
    // Use physical size for initial Vulkan setup
    let extent = vk::Extent2D { width: primary_window.physical_width(), height: primary_window.physical_height() };

    // Lock the context to pass to Renderer::new
    let mut vk_ctx_guard = vk_context_res.0.lock().expect("Failed to lock VulkanContext for renderer creation");

    // Create the *actual* renderer instance
    let renderer_instance = Renderer::new(&mut vk_ctx_guard, extent); // Pass &mut guard
    info!("Actual Renderer instance created (Core Plugin).");

    // Wrap in Arc<Mutex> and insert as resource
    let renderer_arc = Arc::new(Mutex::new(renderer_instance));
    commands.insert_resource(RendererResource(renderer_arc.clone()));

    info!("Renderer resource created and inserted (Core Plugin).");
}

/// Update system: Handles window resize events.
fn handle_resize_system(
    mut resize_reader: EventReader<bevy_window::WindowResized>,
    renderer_res_opt: Option<ResMut<RendererResource>>, // Use ResMut
    vk_context_res_opt: Option<Res<VulkanContextResource>>,
) {
    // Use Option pattern to gracefully handle missing resources during shutdown
    let Some(renderer_res) = renderer_res_opt else { return; };
    let Some(vk_context_res) = vk_context_res_opt else { return; };

    for event in resize_reader.read() {
        info!("WindowResized event (Core Plugin): {:?}", event);
        // Use logical size for resize logic (Renderer::resize_renderer expects logical)
        if event.width > 0.0 && event.height > 0.0 {
            if let (Ok(mut renderer_guard), Ok(mut vk_ctx_guard)) = (
                renderer_res.0.lock(),
                vk_context_res.0.lock(),
            ) {
                info!("Calling actual resize logic (Core Plugin).");
                // Pass logical width/height
                renderer_guard.resize_renderer(&mut vk_ctx_guard, event.width as u32, event.height as u32);
            } else {
                warn!("Could not lock resources for resize handling (Core Plugin).");
            }
        }
    }
}

/// Last system: Triggers rendering via the custom Vulkan Renderer.
fn rendering_system(
    renderer_res_opt: Option<ResMut<RendererResource>>, // Use ResMut
    vk_context_res_opt: Option<Res<VulkanContextResource>>,
    // Query for renderable entities using ECS components
    query: Query<(Entity, &GlobalTransform, &ShapeData, &Visibility)>,
    // Query for entities whose ShapeData component changed this frame
    shapes_query: Query<Entity, (With<Visibility>, Changed<ShapeData>)>,
) {
    // Use Option pattern to gracefully handle missing resources during shutdown
    if let (Some(renderer_res), Some(vk_context_res)) =
        (renderer_res_opt, vk_context_res_opt)
    {
        // --- Collect IDs of entities whose ShapeData changed ---
        let changed_entities: HashSet<Entity> = shapes_query.iter().collect();
        // Optional: Log only if changes occurred
        // if !changed_entities.is_empty() {
        //     info!("Detected ShapeData changes for entities: {:?}", changed_entities);
        // }

        // --- Collect Render Data from ECS ---
        let mut render_commands: Vec<RenderCommandData> = Vec::new();
        for (entity, global_transform, shape, visibility) in query.iter() {
            if visibility.is_visible() { // Use method on custom Visibility
                let vertices_changed = changed_entities.contains(&entity);
                render_commands.push(RenderCommandData {
                    entity_id: entity,
                    transform_matrix: global_transform.compute_matrix(),
                    vertices: shape.vertices.clone(), // Clone Arc
                    vertex_shader_path: shape.vertex_shader_path.clone(),
                    fragment_shader_path: shape.fragment_shader_path.clone(),
                    depth: global_transform.translation().z,
                    vertices_changed,
                });
            }
        }

        // Sort render_commands by depth (higher Z drawn later/on top)
        render_commands.sort_unstable_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap_or(std::cmp::Ordering::Equal));

        // --- Call Custom Renderer ---
        if let (Ok(mut renderer_guard), Ok(mut vk_ctx_guard)) = (
            renderer_res.0.lock(),
            vk_context_res.0.lock(),
        ) {
            renderer_guard.render(&mut vk_ctx_guard, &render_commands); // Pass &mut guard and commands
        } else {
            warn!("Could not lock resources for rendering trigger (Core Plugin).");
        }
    }
}


/// System running on AppExit in Update schedule: Takes ownership of Vulkan/Renderer resources via World access and cleans them up immediately.
fn cleanup_trigger_system(world: &mut World) {
    info!("ENTERED cleanup_trigger_system (Core Plugin on AppExit)");

    // --- Take Ownership of Resources ---
    let renderer_res_opt: Option<RendererResource> = world.remove_resource::<RendererResource>();
    let vk_context_res_opt: Option<VulkanContextResource> = world.remove_resource::<VulkanContextResource>();

    // --- Perform Cleanup ---
    if let Some(vk_context_res) = vk_context_res_opt {
        info!("VulkanContextResource taken (Core Plugin).");
        match vk_context_res.0.lock() {
            Ok(mut vk_ctx_guard) => {
                info!("Successfully locked VulkanContext Mutex (Core Plugin).");

                // 1. Cleanup Renderer (if it existed)
                if let Some(renderer_res) = renderer_res_opt {
                    info!("RendererResource taken (Core Plugin).");
                    match renderer_res.0.lock() {
                        Ok(mut renderer_guard) => {
                            info!("Successfully locked Renderer Mutex (Core Plugin).");
                            info!("Calling actual Renderer cleanup via MutexGuard (Core Plugin).");
                            renderer_guard.cleanup(&mut vk_ctx_guard); // Pass vk_ctx guard
                        }
                        Err(poisoned) => {
                            error!("Renderer Mutex was poisoned before cleanup (Core Plugin): {:?}", poisoned);
                        }
                    }
                } else {
                    info!("Renderer resource not found or already removed (Core Plugin).");
                }

                // 2. Cleanup Vulkan Context
                info!("Calling cleanup_vulkan (Core Plugin)...");
                cleanup_vulkan(&mut vk_ctx_guard); // Pass vk_ctx guard
                info!("cleanup_vulkan finished (Core Plugin).");

            } // vk_ctx_guard dropped here
            Err(poisoned) => {
                error!("VulkanContext Mutex was poisoned before cleanup (Core Plugin): {:?}", poisoned);
            }
        }
    } else {
        warn!("VulkanContext resource not found or already removed during cleanup trigger (Core Plugin).");
    }

    // --- Resources go out of scope ---
    info!("Cleanup trigger system finished (Core Plugin), taken resources going out of scope.");
}