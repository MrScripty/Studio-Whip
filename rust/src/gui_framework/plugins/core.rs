use bevy_app::{App, AppExit, Plugin, Startup, Update, Last};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{SystemSet, common_conditions::{not, on_event}};
use bevy_log::{info, error, warn};
use bevy_window::{PrimaryWindow, Window};
use bevy_winit::WinitWindows;
use bevy_transform::prelude::{GlobalTransform, Transform}; // Added GlobalTransform
use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use ash::vk;
use bevy_color::Color;
use bevy_math::{Vec2, IVec2};
use cosmic_text::{Attrs, BufferLine, FontSystem, Metrics, Shaping, SwashCache, Wrap, Color as CosmicColor};

// Import types from the crate root (lib.rs)
use crate::{Vertex, RenderCommandData, TextVertex, TextRenderCommandData}; // Added TextVertex, TextRenderCommandData

// Import types/functions from the gui_framework
use crate::gui_framework::{
    context::vulkan_setup::{setup_vulkan, cleanup_vulkan},
    rendering::render_engine::Renderer,
    rendering::glyph_atlas::{GlyphAtlas, GlyphInfo},
    rendering::font_server::FontServer,
    components::{ShapeData, Visibility, Text, FontId, TextAlignment, TextLayoutOutput, PositionedGlyph},
};

// Import resources used/managed by this plugin's systems
use crate::{VulkanContextResource, RendererResource, GlyphAtlasResource, FontServerResource, SwashCacheResource};

// --- System Sets ---
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoreSet {
    SetupVulkan,
    CreateRenderer,
    CreateGlyphAtlas,
    CreateFontServer,
    CreateSwashCache,
    HandleResize,
    TextLayout,
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
        // Register math types used in reflection
        app.register_type::<Vec2>();
        app.register_type::<IVec2>();


        // --- System Setup ---
        app
            // == Startup Systems ==
            .configure_sets(Startup,(
                CoreSet::SetupVulkan,
                CoreSet::CreateRenderer,
                CoreSet::CreateGlyphAtlas,
                CoreSet::CreateFontServer,
                CoreSet::CreateSwashCache,
            ).chain()
        )
            .add_systems(Startup, (
                setup_vulkan_system.in_set(CoreSet::SetupVulkan),
                create_renderer_system.in_set(CoreSet::CreateRenderer),
                create_glyph_atlas_system.in_set(CoreSet::CreateGlyphAtlas),
                create_font_server_system.in_set(CoreSet::CreateFontServer),
                create_swash_cache_system.in_set(CoreSet::CreateSwashCache),
        ))
            // == Update Systems ==
            .configure_sets(Update, (
                CoreSet::HandleResize,
                CoreSet::TextLayout.after(CoreSet::HandleResize),
        ))
        .add_systems(Update, ( // <-- Correct: This tuple contains the systems
            handle_resize_system.in_set(CoreSet::HandleResize),
            text_layout_system.in_set(CoreSet::TextLayout),
        ))
            .configure_sets(Last,
                // Ensure Render runs after TextLayout
                CoreSet::Render.after(CoreSet::TextLayout)
        )
            // == Rendering System (runs late) ==
            .add_systems(Last, (
                rendering_system.run_if(not(on_event::<AppExit>)).in_set(CoreSet::Render),
                cleanup_trigger_system.run_if(on_event::<AppExit>).in_set(CoreSet::Cleanup).after(CoreSet::Render),
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

fn create_font_server_system(mut commands: Commands) {
    info!("Running create_font_server_system (Core Plugin)...");
    // FontServer::new() can take some time if loading many system fonts.
    // Consider running this asynchronously or loading fewer fonts if startup time is critical.
    let font_server = FontServer::new();
    let font_server_arc = Arc::new(Mutex::new(font_server));
    commands.insert_resource(FontServerResource(font_server_arc));
    info!("FontServer resource created and inserted (Core Plugin).");
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

fn create_swash_cache_system(mut commands: Commands) {
    info!("Running create_swash_cache_system (Core Plugin)...");
    let swash_cache = SwashCache::new(); // SwashCache::new() is available
    commands.insert_resource(SwashCacheResource(Mutex::new(swash_cache)));
    info!("SwashCache resource created and inserted (Core Plugin).");
}

fn text_layout_system(
    mut commands: Commands,
    query: Query<(Entity, &Text, &Transform), (Changed<Text>, With<Visibility>)>,
    font_server_res: Res<FontServerResource>,
    glyph_atlas_res: Res<GlyphAtlasResource>,
    swash_cache_res: Res<SwashCacheResource>,
    vk_context_res: Res<VulkanContextResource>,
) {
    let Ok(mut font_server) = font_server_res.0.lock() else {
        error!("Failed to lock FontServerResource in text_layout_system");
        return;
    };
    let Ok(mut glyph_atlas) = glyph_atlas_res.0.lock() else {
        error!("Failed to lock GlyphAtlasResource in text_layout_system");
        return;
    };
    let Ok(mut swash_cache) = swash_cache_res.0.lock() else {
        error!("Failed to lock SwashCacheResource in text_layout_system");
        return;
    };
    let Ok(vk_context) = vk_context_res.0.lock() else {
        error!("Failed to lock VulkanContextResource in text_layout_system");
        return;
    };

    for (entity, text, transform) in query.iter() {
        let metrics = Metrics::new(text.size, text.size * 1.2);
        let mut buffer = cosmic_text::Buffer::new(&mut font_server.font_system, metrics);

        let cosmic_color = match text.color {
            Color::Srgba(rgba) => CosmicColor::rgba(
                (rgba.red * 255.0) as u8,
                (rgba.green * 255.0) as u8,
                (rgba.blue * 255.0) as u8,
                (rgba.alpha * 255.0) as u8,
            ),
            _ => {
                warn!("Unsupported Bevy color type encountered in text layout, defaulting.");
                CosmicColor::rgb(255, 255, 255)
            }
        };
        let attrs = Attrs::new().color(cosmic_color);
        buffer.set_text(&mut font_server.font_system, &text.content, &attrs, Shaping::Advanced);

        if let Some(bounds) = text.bounds {
            buffer.set_size(&mut font_server.font_system, Some(bounds.x), Some(bounds.y));
            buffer.set_wrap(&mut font_server.font_system, Wrap::Word);
        } else {
            buffer.set_size(&mut font_server.font_system, None, None);
            buffer.set_wrap(&mut font_server.font_system, Wrap::None);
        }

        buffer.shape_until_scroll(&mut font_server.font_system, true);

        let mut positioned_glyphs = Vec::new();

        for run in buffer.layout_runs() {
            // Calculate the baseline for this line (convert Y-down to Y-up)
            let baseline_y = -run.line_y;

            for layout_glyph in run.glyphs.iter() {
                let flags = cosmic_text::CacheKeyFlags::empty();
                let (cache_key, _x_int_offset, _y_int_offset) = cosmic_text::CacheKey::new(
                    layout_glyph.font_id,
                    layout_glyph.glyph_id,
                    layout_glyph.font_size,
                    (layout_glyph.x, layout_glyph.y),
                    flags,
                );

                let Some(swash_image) = swash_cache.get_image(&mut font_server.font_system, cache_key) else {
                    warn!("Failed to get swash image for glyph key: {:?}", cache_key);
                    continue;
                };

                match glyph_atlas.add_glyph(&vk_context, cache_key, &swash_image) {
                    Ok(glyph_info_ref) => {
                        let glyph_info = *glyph_info_ref;
                        let placement = swash_image.placement;
                        let width = placement.width as f32;
                        let height = placement.height as f32;

                        // Get the font to retrieve metrics
                        let font = match font_server.font_system.get_font(layout_glyph.font_id) {
                            Some(font) => font,
                            None => {
                                warn!("Font with ID {:?} not found for glyph {:?}", layout_glyph.font_id, layout_glyph.glyph_id);
                                continue;
                            }
                        };

                        // Create a FontRef for swash to access metrics
                        let font_ref = swash::FontRef {
                            data: font.as_ref().data(),
                            offset: 0, // Assuming single font face; adjust if using font collections
                            key: Default::default(),
                        };

                        // Get the font metrics (unscaled)
                        let metrics = font_ref.metrics(&[]); // No variations
                        // Scale the metrics to the font size
                        let units_per_em = metrics.units_per_em as f32;
                        if units_per_em == 0.0 {
                            warn!("Units per em is 0 for font ID {:?}, defaulting to ascent 0", layout_glyph.font_id);
                            continue;
                        }
                        let scale_factor = layout_glyph.font_size / units_per_em;
                        let ascent = metrics.ascent * scale_factor; // Ascent is positive upward in swash (Y-up)
                        let descent = metrics.descent * scale_factor; // Descent is negative downward in swash (Y-up)

                        // Position the quad such that its baseline is at baseline_y
                        // placement.top is the distance from the baseline to the top of the bitmap (positive downward in Y-down)
                        // In Y-up, the top of the bitmap is at baseline_y + placement.top
                        let relative_top_y = baseline_y + placement.top as f32;
                        let relative_bottom_y = relative_top_y - height;

                        // Horizontal positioning (unchanged)
                        let relative_left_x = layout_glyph.x;
                        let relative_right_x = relative_left_x + width;

                        // Define quad vertices (TL, TR, BR, BL order)
                        let top_left = Vec2::new(relative_left_x, relative_top_y);
                        let top_right = Vec2::new(relative_right_x, relative_top_y);
                        let bottom_right = Vec2::new(relative_right_x, relative_bottom_y);
                        let bottom_left = Vec2::new(relative_left_x, relative_bottom_y);

                        let relative_vertices = [top_left, top_right, bottom_right, bottom_left];

                        // Debug logging to verify positioning
                        info!(
                            "Glyph {:?}: baseline_y={:.2}, placement_top={:.2}, top_y={:.2}, bottom_y={:.2}, ascent={:.2}, descent={:.2}",
                            layout_glyph.glyph_id, baseline_y, placement.top as f32, relative_top_y, relative_bottom_y, ascent, descent
                        );

                        positioned_glyphs.push(PositionedGlyph {
                            glyph_info,
                            layout_glyph: layout_glyph.clone(),
                            vertices: relative_vertices,
                        });
                    }
                    Err(e) => {
                        warn!(
                            "Failed to add/get glyph (key: {:?}) from atlas: {}. Skipping glyph.",
                            cache_key, e
                        );
                    }
                }
            }
        }

        commands.entity(entity).insert(TextLayoutOutput {
            glyphs: positioned_glyphs,
        });
    }
}

// Last system: Triggers rendering via the custom Vulkan Renderer.
fn rendering_system(
    renderer_res_opt: Option<ResMut<RendererResource>>,
    vk_context_res_opt: Option<Res<VulkanContextResource>>,
    // Query for shapes
    shape_query: Query<(Entity, &GlobalTransform, &ShapeData, &Visibility), Without<TextLayoutOutput>>, // Exclude text entities
    shape_change_query: Query<Entity, (With<Visibility>, Changed<ShapeData>)>,
    // Query for text entities that have layout results
    text_query: Query<(Entity, &GlobalTransform, &TextLayoutOutput, &Visibility)>, // Query layout output
    // Resources
    glyph_atlas_res: Option<Res<GlyphAtlasResource>>, // Need atlas for texture binding
) {
    // Ensure all required resources are available
    if let (Some(renderer_res), Some(vk_context_res), Some(atlas_res)) =
        (renderer_res_opt, vk_context_res_opt, glyph_atlas_res)
    {
        // --- Collect Shape Render Data ---
        let changed_shape_entities: HashSet<Entity> = shape_change_query.iter().collect();
        let mut shape_render_commands: Vec<RenderCommandData> = Vec::new();
        for (entity, global_transform, shape, visibility) in shape_query.iter() {
            if visibility.is_visible() {
                let vertices_changed = changed_shape_entities.contains(&entity);
                shape_render_commands.push(RenderCommandData {
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
        shape_render_commands.sort_unstable_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap_or(std::cmp::Ordering::Equal));

        // --- Collect Text Render Data ---
        let mut all_text_vertices: Vec<TextVertex> = Vec::new();
        for (_entity, global_transform, text_layout, visibility) in text_query.iter() {
            if visibility.is_visible() {
                let transform_matrix = global_transform.compute_matrix();
                for positioned_glyph in &text_layout.glyphs {
                    // Combine relative glyph vertices with entity transform
                    // Vertices order: top-left, top-right, bottom-right, bottom-left (relative Y-down)
                    let world_pos = |rel_pos: Vec2| {
                        // Apply transform. Note: rel_pos.y is negative for top vertices.
                        transform_matrix.transform_point3(rel_pos.extend(0.0)).truncate()
                    };

                    let tl_world = world_pos(positioned_glyph.vertices[0]);
                    let tr_world = world_pos(positioned_glyph.vertices[1]);
                    let br_world = world_pos(positioned_glyph.vertices[2]);
                    let bl_world = world_pos(positioned_glyph.vertices[3]);

                    // Get UVs from GlyphInfo
                    let uv_min = positioned_glyph.glyph_info.uv_min;
                    let uv_max = positioned_glyph.glyph_info.uv_max;

                    // Create vertices for two triangles (quad)
                    // Triangle 1: top-left, bottom-left, bottom-right
                    all_text_vertices.push(TextVertex { position: tl_world.into(), uv: [uv_min[0], uv_min[1]] }); // Top-Left UV
                    all_text_vertices.push(TextVertex { position: bl_world.into(), uv: [uv_min[0], uv_max[1]] }); // Bottom-Left UV
                    all_text_vertices.push(TextVertex { position: br_world.into(), uv: [uv_max[0], uv_max[1]] }); // Bottom-Right UV

                    // Triangle 2: top-left, bottom-right, top-right
                    all_text_vertices.push(TextVertex { position: tl_world.into(), uv: [uv_min[0], uv_min[1]] }); // Top-Left UV
                    all_text_vertices.push(TextVertex { position: br_world.into(), uv: [uv_max[0], uv_max[1]] }); // Bottom-Right UV
                    all_text_vertices.push(TextVertex { position: tr_world.into(), uv: [uv_max[0], uv_min[1]] }); // Top-Right UV
                }
            }
        }

        // --- Call Custom Renderer ---
        if let (Ok(mut renderer_guard), Ok(mut vk_ctx_guard)) = (renderer_res.0.lock(), vk_context_res.0.lock()) {
            // Pass shape commands, collected text vertices, and atlas resource
            renderer_guard.render(
                &mut vk_ctx_guard,
                &shape_render_commands,
                &all_text_vertices,
                &atlas_res, // Pass the GlyphAtlasResource
            );
        } else { warn!("Could not lock resources for rendering trigger (Core Plugin)."); }
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