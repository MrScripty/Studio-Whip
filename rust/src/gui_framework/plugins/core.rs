use bevy_app::{App, AppExit, Plugin, Startup, Update, Last};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{SystemSet, common_conditions::{not, on_event}};
use bevy_log::{info, error, warn};
use bevy_window::{PrimaryWindow, Window};
use bevy_winit::WinitWindows;
use bevy_transform::prelude::GlobalTransform;
use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use ash::vk;
use bevy_color::Color;

// Import types from the crate root (lib.rs)
use crate::{Vertex, RenderCommandData};

// Import types/functions from the gui_framework
use crate::gui_framework::{
    context::vulkan_setup::{setup_vulkan, cleanup_vulkan},
    rendering::render_engine::Renderer,
    rendering::glyph_atlas::GlyphAtlas,
    components::{ShapeData, Visibility, Text, FontId, TextAlignment},
};

// Import resources used/managed by this plugin's systems
use crate::{VulkanContextResource, RendererResource, GlyphAtlasResource};

// --- System Sets ---
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoreSet {
    SetupVulkan,
    CreateRenderer,
    CreateGlyphAtlas,
    HandleResize,
    Render,
    Cleanup,
}

// --- Core Plugin Definition ---
pub struct GuiFrameworkCorePlugin;

impl Plugin for GuiFrameworkCorePlugin {
    fn build(&self, app: &mut App) {
        info!("Building GuiFrameworkCorePlugin...");

        // --- Type Registration ---
        app.register_type::<ShapeData>();
        app.register_type::<Visibility>();
        app.register_type::<Vertex>();
        // Note: Resources like GlyphAtlasResource don't need registration if they derive Resource
        // Register new Text component and related types
        app.register_type::<Text>();
        app.register_type::<FontId>();
        app.register_type::<TextAlignment>();
        app.register_type::<Color>();


        // --- System Setup ---
        app
            // == Startup Systems ==
            .configure_sets(Startup,(
                CoreSet::SetupVulkan, 
                CoreSet::CreateRenderer, 
                CoreSet::CreateGlyphAtlas,
            ).chain()
        )
            .add_systems(Startup, (
                setup_vulkan_system.in_set(CoreSet::SetupVulkan),
                create_renderer_system.in_set(CoreSet::CreateRenderer),
                create_glyph_atlas_system.in_set(CoreSet::CreateGlyphAtlas),
        ))
            // == Update Systems ==
            .configure_sets(Update,
                CoreSet::HandleResize
        )
            .add_systems(Update,
                handle_resize_system.in_set(CoreSet::HandleResize)
        )
            // == Rendering System (runs late) ==
            .add_systems(Last, (
                rendering_system.run_if(not(on_event::<AppExit>)).in_set(CoreSet::Render),
                cleanup_trigger_system.run_if(on_event::<AppExit>).in_set(CoreSet::Cleanup),
        ));
        info!("GuiFrameworkCorePlugin built.");
    }
}


// --- Systems Moved from main.rs ---

// Startup system: Initializes Vulkan using the primary window handle.
fn setup_vulkan_system(
    vk_context_res: Res<VulkanContextResource>,
    primary_window_q: Query<Entity, With<PrimaryWindow>>,
    winit_windows: NonSend<WinitWindows>,
) {
    info!("Running setup_vulkan_system (Core Plugin)...");
    let primary_entity = primary_window_q.get_single()
        .expect("Failed to get primary window entity");
    let winit_window = winit_windows.get_window(primary_entity)
        .expect("Failed to get winit window reference from WinitWindows");

    let mut vk_ctx_guard = vk_context_res.0.lock().expect("Failed to lock VulkanContext mutex for setup");

    setup_vulkan(&mut vk_ctx_guard, winit_window);
    info!("Vulkan setup complete (Core Plugin).");
}

// Startup system (piped): Creates the Renderer instance resource.
fn create_renderer_system(
    mut commands: Commands,
    vk_context_res: Res<VulkanContextResource>,
    primary_window_q: Query<&Window, With<PrimaryWindow>>,
) {
    info!("Running create_renderer_system (Core Plugin)...");

    let primary_window = primary_window_q.get_single().expect("Primary window not found");
    let extent = vk::Extent2D { width: primary_window.physical_width(), height: primary_window.physical_height() };

    let mut vk_ctx_guard = vk_context_res.0.lock().expect("Failed to lock VulkanContext for renderer creation");

    let renderer_instance = Renderer::new(&mut vk_ctx_guard, extent);
    info!("Actual Renderer instance created (Core Plugin).");

    let renderer_arc = Arc::new(Mutex::new(renderer_instance));
    commands.insert_resource(RendererResource(renderer_arc.clone()));

    info!("Renderer resource created and inserted (Core Plugin).");
}

fn create_glyph_atlas_system(
    mut commands: Commands,
    vk_context_res: Res<VulkanContextResource>,
) {
    info!("Running create_glyph_atlas_system (Core Plugin)...");
    let mut vk_ctx_guard = vk_context_res.0.lock().expect("Failed to lock VulkanContext for glyph atlas creation");

    // Choose an initial size for the atlas
    let initial_extent = vk::Extent2D { width: 1024, height: 1024 };

    match GlyphAtlas::new(&mut vk_ctx_guard, initial_extent) {
        Ok(atlas) => {
            let atlas_arc = Arc::new(Mutex::new(atlas));
            commands.insert_resource(GlyphAtlasResource(atlas_arc));
            info!("GlyphAtlas resource created and inserted (Core Plugin).");
        }
        Err(e) => {
            // Use expect here because atlas is critical for text rendering
            panic!("Failed to create GlyphAtlas: {}", e);
        }
    }
}

// Update system: Handles window resize events.
fn handle_resize_system(
    mut resize_reader: EventReader<bevy_window::WindowResized>,
    renderer_res_opt: Option<ResMut<RendererResource>>,
    vk_context_res_opt: Option<Res<VulkanContextResource>>,
) {
    let Some(renderer_res) = renderer_res_opt else { return; };
    let Some(vk_context_res) = vk_context_res_opt else { return; };

    for event in resize_reader.read() {
        info!("WindowResized event (Core Plugin): {:?}", event);
        if event.width > 0.0 && event.height > 0.0 {
            if let (Ok(mut renderer_guard), Ok(mut vk_ctx_guard)) = (
                renderer_res.0.lock(),
                vk_context_res.0.lock(),
            ) {
                info!("Calling actual resize logic (Core Plugin).");
                renderer_guard.resize_renderer(&mut vk_ctx_guard, event.width as u32, event.height as u32);
            } else {
                warn!("Could not lock resources for resize handling (Core Plugin).");
            }
        }
    }
}

// Last system: Triggers rendering via the custom Vulkan Renderer.
fn rendering_system(
    renderer_res_opt: Option<ResMut<RendererResource>>,
    vk_context_res_opt: Option<Res<VulkanContextResource>>,
    query: Query<(Entity, &GlobalTransform, &ShapeData, &Visibility)>,
    shapes_query: Query<Entity, (With<Visibility>, Changed<ShapeData>)>,
    // text_query: Query<(Entity, &GlobalTransform, &Text, &Visibility)>,
    // glyph_atlas_res: Option<Res<GlyphAtlasResource>>, // Will need this later
) {
    if let (Some(renderer_res), Some(vk_context_res)) =
        (renderer_res_opt, vk_context_res_opt)
    {
        let changed_entities: HashSet<Entity> = shapes_query.iter().collect();

        let mut render_commands: Vec<RenderCommandData> = Vec::new();
        for (entity, global_transform, shape, visibility) in query.iter() {
            if visibility.is_visible() {
                let vertices_changed = changed_entities.contains(&entity);
                render_commands.push(RenderCommandData {
                    entity_id: entity,
                    transform_matrix: global_transform.compute_matrix(),
                    vertices: shape.vertices.clone(),
                    vertex_shader_path: shape.vertex_shader_path.clone(),
                    fragment_shader_path: shape.fragment_shader_path.clone(),
                    depth: global_transform.translation().z,
                    vertices_changed,
                });
            }
        }

        // --- TODO: Collect Text Render Data ---
        // This will involve:
        // 1. Querying text entities.
        // 2. Accessing the GlyphAtlasResource.
        // 3. Calling glyph_atlas.add_glyph() for needed glyphs (triggering rasterization/upload if new).
        // 4. Generating vertex data for text quads using GlyphInfo UVs.
        // 5. Passing text vertex data + atlas texture to the renderer.

        render_commands.sort_unstable_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap_or(std::cmp::Ordering::Equal));

        if let (Ok(mut renderer_guard), Ok(mut vk_ctx_guard)) = (
            renderer_res.0.lock(),
            vk_context_res.0.lock(),
        ) {
            renderer_guard.render(&mut vk_ctx_guard, &render_commands);
        } else {
            warn!("Could not lock resources for rendering trigger (Core Plugin).");
        }
    }
}


// System running on AppExit in Last schedule: Takes ownership of Vulkan/Renderer resources via World access and cleans them up immediately.
fn cleanup_trigger_system(world: &mut World) {
    info!("ENTERED cleanup_trigger_system (Core Plugin on AppExit)");

    let renderer_res_opt: Option<RendererResource> = world.remove_resource::<RendererResource>();
    let vk_context_res_opt: Option<VulkanContextResource> = world.remove_resource::<VulkanContextResource>();
    let glyph_atlas_res_opt: Option<GlyphAtlasResource> = world.remove_resource::<GlyphAtlasResource>();

    if let Some(vk_context_res) = vk_context_res_opt {
        info!("VulkanContextResource taken (Core Plugin).");
        match vk_context_res.0.lock() {
            Ok(mut vk_ctx_guard) => {
                info!("Successfully locked VulkanContext Mutex (Core Plugin).");

                // 1. Cleanup Renderer
                if let Some(renderer_res) = renderer_res_opt {
                    info!("RendererResource taken (Core Plugin).");
                    match renderer_res.0.lock() {
                        Ok(mut renderer_guard) => {
                            info!("Successfully locked Renderer Mutex (Core Plugin).");
                            info!("Calling actual Renderer cleanup via MutexGuard (Core Plugin).");
                            renderer_guard.cleanup(&mut vk_ctx_guard);
                        }
                        Err(poisoned) => {
                            error!("Renderer Mutex was poisoned before cleanup (Core Plugin): {:?}", poisoned);
                        }
                    }
                } else { info!("Renderer resource not found or already removed (Core Plugin)."); }

                // 2. Cleanup Glyph Atlas
                if let Some(atlas_res) = glyph_atlas_res_opt {
                    info!("GlyphAtlasResource taken (Core Plugin).");
                     match atlas_res.0.lock() {
                        Ok(mut atlas_guard) => {
                            info!("Successfully locked GlyphAtlas Mutex (Core Plugin).");
                            // Pass immutable borrow of vk_ctx_guard needed for cleanup
                            atlas_guard.cleanup(&vk_ctx_guard);
                        }
                        Err(poisoned) => error!("GlyphAtlas Mutex poisoned: {:?}", poisoned),
                    }
                } else { info!("GlyphAtlas resource not found (Core Plugin)."); }

                // 3. Cleanup Vulkan Context (Must be last after resources using it are cleaned)
                info!("Calling cleanup_vulkan (Core Plugin)...");
                cleanup_vulkan(&mut vk_ctx_guard);
                info!("cleanup_vulkan finished (Core Plugin).");

            }
            Err(poisoned) => {
                error!("VulkanContext Mutex was poisoned before cleanup (Core Plugin): {:?}", poisoned);
            }
        }
    } else {
        warn!("VulkanContext resource not found or already removed during cleanup trigger (Core Plugin).");
    }

    info!("Cleanup trigger system finished (Core Plugin), taken resources going out of scope.");
}