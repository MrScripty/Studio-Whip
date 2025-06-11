use bevy_app::{App, AppExit, Plugin, Startup, Update, Last};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{SystemSet, common_conditions::{not, on_event}};
use bevy_log::{info, error, warn};
use bevy_window::{PrimaryWindow, Window};
use bevy_winit::WinitWindows;
use bevy_transform::prelude::{GlobalTransform, Transform};
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, HashSet};
use ash::vk;
use bevy_color::Color;
use bevy_math::{Vec2, IVec2, Mat4};
use cosmic_text::{Attrs, Shaping, SwashCache, Wrap, Color as CosmicColor, Font, Buffer, Metrics};
use swash::FontRef;
use vk_mem::Alloc;
use yrs::{Transact, GetString, TextRef};
use bevy_hierarchy::{BuildChildren, DespawnRecursiveExt, Children, Parent};
use bevy_core::Name;
use crate::gui_framework::plugins::interaction::InteractionSet;
use crate::BufferManagerResource;
use crate::gui_framework::context::vulkan_setup::set_debug_object_name;
use crate::gui_framework::interaction::utils::global_to_local_cursor;

// Import types from the crate root (lib.rs)
use crate::{
    Vertex, RenderCommandData, TextVertex,
    GlobalProjectionUboResource,
    TextRenderingResources, // Keep this if cleanup needs it, otherwise remove
    YrsDocResource,
};

// Import types/functions from the gui_framework
use crate::gui_framework::events::YrsTextChanged;
use crate::gui_framework::{
    context::vulkan_context::VulkanContext,
    context::vulkan_setup::{setup_vulkan, cleanup_vulkan},
    rendering::render_engine::Renderer,
    rendering::glyph_atlas::GlyphAtlas,
    rendering::font_server::FontServer,
    components::{ShapeData, Visibility, Text, FontId, TextAlignment, TextLayoutOutput, PositionedGlyph, TextBufferCache, TextSelection, Focus, Interaction, CursorVisual, CursorState},
    rendering::shader_utils,
};

// Import resources used/managed by this plugin's systems
use crate::{VulkanContextResource, RendererResource, GlyphAtlasResource, FontServerResource, SwashCacheResource};

// --- System Sets ---
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoreSet {
    // Startup sequence
    SetupVulkan,
    CreateGlobalUbo,
    CreateRenderer,         // needs Vulkan setup
    CreateGlyphAtlas,       // needs Vulkan setup
    CreateFontServer,       // no Vulkan deps
    CreateSwashCache,       // no Vulkan deps
    CreateTextResources,    // Create text Vulkan resources (needs Renderer, Atlas)

    // Update sequence
    ApplyUpdateCommands,    // Apply commands from layout/cursor systems before rendering
    HandleResize,           // Handle window resize events, update global UBO
    ApplyInputCommands,     // Apply commands from input/focus/cursor systems before layout/positioning
    TextLayout,             // Perform text layout using cosmic-text
    ManageCursorVisual,     // Spawn/despawn cursor visual based on Focus
    UpdateCursorTransform,  // Update cursor visual position based on state/layout


    // Last sequence
    PreRenderCleanup,       // New set for despawn cleanup before rendering
    Render,                 // Perform rendering using prepared data
    Cleanup,                // Cleanup resources on AppExit
}

// --- Core Plugin Definition ---
pub struct GuiFrameworkCorePlugin;

impl Plugin for GuiFrameworkCorePlugin {
    fn build(&self, app: &mut App) {
        // --- Type Registration ---
        app.register_type::<ShapeData>();
        app.register_type::<Visibility>();
        app.register_type::<Vertex>();
        app.register_type::<Text>();
        app.register_type::<FontId>();
        app.register_type::<TextAlignment>();
        app.register_type::<Color>();
        app.register_type::<Vec2>();
        app.register_type::<IVec2>();
        app.register_type::<crate::gui_framework::components::CursorState>();
        app.register_type::<crate::gui_framework::components::CursorVisual>();

        // --- System Setup ---
        app
            // == Startup Systems ==
            .configure_sets(Startup,(
                CoreSet::SetupVulkan,
                CoreSet::CreateRenderer,
                CoreSet::CreateGlyphAtlas,
                CoreSet::CreateFontServer,
                CoreSet::CreateSwashCache,
            ).chain())
            .configure_sets(Startup, (
                CoreSet::CreateGlobalUbo.after(CoreSet::CreateRenderer),
                CoreSet::CreateTextResources
                    .after(CoreSet::CreateRenderer)
                    .after(CoreSet::CreateGlyphAtlas),
            ))
            .add_systems(Startup, (
                setup_vulkan_system.in_set(CoreSet::SetupVulkan),
                create_renderer_system.in_set(CoreSet::CreateRenderer),
                create_glyph_atlas_system.in_set(CoreSet::CreateGlyphAtlas),
                create_font_server_system.in_set(CoreSet::CreateFontServer),
                create_swash_cache_system.in_set(CoreSet::CreateSwashCache),
                create_global_ubo_system.in_set(CoreSet::CreateGlobalUbo),
                create_text_rendering_resources_system.in_set(CoreSet::CreateTextResources),
            ))
            .init_resource::<crate::PreparedTextDrawsResource>();

            // == Update Systems (This is the corrected ordering) ==
            app.configure_sets(Update, (
                // Phase 1: Apply commands from input handling. This makes component changes
                // (like adding `Focus`) available to the rest of the frame.
                CoreSet::ApplyInputCommands
                    .after(InteractionSet::InputHandling),

                // Phase 2: Systems that react to the newly applied components and events.
                // Both of these must run after commands are applied.
                CoreSet::TextLayout
                    .after(CoreSet::ApplyInputCommands),
                CoreSet::ManageCursorVisual
                    .after(CoreSet::ApplyInputCommands),

                // Phase 3: The cursor's visual transform can only be updated *after* the
                // layout has been calculated AND the visual entity has been spawned.
                CoreSet::UpdateCursorTransform
                    .after(CoreSet::TextLayout)
                    .after(CoreSet::ManageCursorVisual),
                
                // Phase 4: The rest of the systems run in a simple linear order.
                CoreSet::HandleResize
                    .after(CoreSet::UpdateCursorTransform),
                CoreSet::PreRenderCleanup
                    .after(CoreSet::HandleResize),
            ))
            .add_systems(Update, (
                // Add all the systems to the schedule. Their sets define the order.
                handle_resize_system.in_set(CoreSet::HandleResize),
                text_layout_system.in_set(CoreSet::TextLayout),
                manage_cursor_visual_system.in_set(CoreSet::ManageCursorVisual),
                update_cursor_transform_system.in_set(CoreSet::UpdateCursorTransform),
                apply_deferred.in_set(CoreSet::ApplyInputCommands),
                buffer_manager_despawn_cleanup_system.in_set(CoreSet::PreRenderCleanup),
            ));

            // == Last Schedule Systems (This part is correct and remains unchanged) ==
            app.configure_sets(Last, (
                CoreSet::Render.after(CoreSet::PreRenderCleanup),
                CoreSet::Cleanup.after(CoreSet::Render),
            ))
            .add_systems(Last, (
                rendering_system.run_if(not(on_event::<AppExit>)).in_set(CoreSet::Render),
                cleanup_trigger_system.run_if(on_event::<AppExit>).in_set(CoreSet::Cleanup),
            ));
    }
}

// Startup system: Initializes Vulkan using the primary window handle.
fn setup_vulkan_system(
    vk_context_res: Res<VulkanContextResource>,
    primary_window_q: Query<Entity, With<PrimaryWindow>>,
    winit_windows: NonSend<WinitWindows>,
) {
    let primary_entity = primary_window_q.get_single()
        .expect("Failed to get primary window entity");
    let winit_window = winit_windows.get_window(primary_entity)
        .expect("Failed to get winit window reference from WinitWindows");

    let mut vk_ctx_guard = vk_context_res.0.lock().expect("Failed to lock VulkanContext mutex for setup");

    setup_vulkan(&mut vk_ctx_guard, winit_window);
}

// Startup system (piped): Creates the Renderer instance resource.
fn create_renderer_system(
    mut commands: Commands,
    vk_context_res: Res<VulkanContextResource>,
    primary_window_q: Query<&Window, With<PrimaryWindow>>,
) {
    let primary_window = primary_window_q.get_single().expect("Primary window not found");
    let extent = vk::Extent2D { width: primary_window.physical_width(), height: primary_window.physical_height() };

    let mut vk_ctx_guard = vk_context_res.0.lock().expect("Failed to lock VulkanContext for renderer creation");

    let renderer_instance = Renderer::new(&mut commands, &mut vk_ctx_guard, extent);

    let renderer_arc = Arc::new(Mutex::new(renderer_instance));
    commands.insert_resource(RendererResource(renderer_arc.clone()));
}

fn buffer_manager_despawn_cleanup_system(
    mut removed_shapes: RemovedComponents<ShapeData>,
    mut removed_cursors: RemovedComponents<CursorVisual>,
    buffer_manager_res_opt: Option<Res<BufferManagerResource>>,
    vk_context_res_opt: Option<Res<VulkanContextResource>>,
) {
    let (Some(buffer_manager_res), Some(vk_context_res)) = (buffer_manager_res_opt, vk_context_res_opt) else {
        // warn!("[BufferManagerCleanup] Missing BufferManagerResource or VulkanContextResource. Skipping cleanup.");
        return;
    };

    let mut entities_to_cleanup: HashSet<Entity> = HashSet::new();
    for entity in removed_shapes.read() { // Use .read() to get iterator
        entities_to_cleanup.insert(entity); // The iterator yields Entity directly
    }
    for entity in removed_cursors.read() { // Use .read() to get iterator
        // CursorVisual entities also use ShapeData, so their resources are managed by BufferManager
        // if they were added to it. This ensures we catch cursors specifically.
        entities_to_cleanup.insert(entity); // The iterator yields Entity directly
    }

    if entities_to_cleanup.is_empty() {
        return;
    }

    let mut bm_guard = match buffer_manager_res.0.lock() {
        Ok(g) => g,
        Err(_) => {
            error!("[BufferManagerCleanup] Failed to lock BufferManagerResource.");
            return;
        }
    };
    let vk_guard = match vk_context_res.0.lock() {
        Ok(g) => g,
        Err(_) => {
            error!("[BufferManagerCleanup] Failed to lock VulkanContextResource.");
            return;
        }
    };

    let Some(device) = vk_guard.device.as_ref() else {
        error!("[BufferManagerCleanup] Vulkan device not available.");
        return;
    };
    let Some(allocator) = vk_guard.allocator.as_ref() else {
        error!("[BufferManagerCleanup] Vulkan allocator not available.");
        return;
    };

    for entity in entities_to_cleanup {
        (*bm_guard).remove_entity_resources(entity, device, allocator);
    }
}

fn create_global_ubo_system(
    mut commands: Commands,
    vk_context_res: Res<VulkanContextResource>,
    renderer_res: Res<RendererResource>, // Need renderer for descriptor pool access
    primary_window_q: Query<&Window, With<PrimaryWindow>>, // Add window query parameter
) {
    info!("Running create_global_ubo_system (Core Plugin)...");
    let Ok(vk_ctx_guard) = vk_context_res.0.lock() else {
        error!("Failed to lock VulkanContext in create_global_ubo_system");
        return;
    };
    let Ok(renderer_guard) = renderer_res.0.lock() else {
        error!("Failed to lock RendererResource in create_global_ubo_system");
        return;
    };
    // Get primary window dimensions
    let Ok(primary_window) = primary_window_q.get_single() else { // Query result assigned to primary_window
        error!("Failed to get primary window in create_global_ubo_system");
        // Cannot proceed without window dimensions for initial projection
        return;
    };
    // Use the 'primary_window' variable obtained from the query
    let initial_logical_width = primary_window.width();
    let initial_logical_height = primary_window.height();
    let device = vk_ctx_guard.device.as_ref().expect("Device missing");
    let allocator = vk_ctx_guard.allocator.as_ref().expect("Allocator missing");

    // 1. Create Buffer & Allocation
    let buffer_size = std::mem::size_of::<Mat4>() as vk::DeviceSize;
    let (buffer, allocation) = unsafe {
        let buffer_info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            size: buffer_size,
            usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let allocation_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            flags: vk_mem::AllocationCreateFlags::MAPPED
            | vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            ..Default::default()
        };
        allocator.create_buffer(&buffer_info, &allocation_info)
        .expect("Global UBO buffer creation failed")
    };
    // --- NAME Global UBO & Memory ---
    #[cfg(debug_assertions)]
    if let Some(debug_device_ext) = vk_ctx_guard.debug_utils_device.as_ref() { // Get Device ext
        let mem_handle = allocator.get_allocation_info(&allocation).device_memory;
        set_debug_object_name(debug_device_ext, buffer, vk::ObjectType::BUFFER, "GlobalProjectionUBO"); // Pass ext
        set_debug_object_name(debug_device_ext, mem_handle, vk::ObjectType::DEVICE_MEMORY, "GlobalProjectionUBO_Mem"); // Pass ext
    }
    // --- END NAME ---

    // 2. Allocate Descriptor Set (using per-entity layout)
    let set_layouts = [renderer_guard.descriptor_set_layout]; // This is now per_entity_layout
    let alloc_info = vk::DescriptorSetAllocateInfo {
        s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
        descriptor_pool: renderer_guard.descriptor_pool, // Access pool field
        descriptor_set_count: 1,
        p_set_layouts: set_layouts.as_ptr(),
        ..Default::default()
    };
    let descriptor_set = unsafe {
        device.allocate_descriptor_sets(&alloc_info)
            .expect("Failed to allocate global UBO descriptor set")
            .remove(0)
    };

    // 3. Update Descriptor Set (Initial Binding)
    let buffer_info = vk::DescriptorBufferInfo {
        buffer,
        offset: 0,
        range: buffer_size,
    };
    let write_set = vk::WriteDescriptorSet {
        s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
        dst_set: descriptor_set,
        dst_binding: 0, // Binding 0 for global projection
        dst_array_element: 0,
        descriptor_count: 1,
        descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
        p_buffer_info: &buffer_info,
        ..Default::default()
    };
    unsafe { device.update_descriptor_sets(&[write_set], &[]); }

    // 4. Initial Write (Perform *before* inserting the resource)
    let proj = Mat4::orthographic_rh(0.0, initial_logical_width, 0.0, initial_logical_height, 1024.0, 0.0);
    let flip_y = Mat4::from_scale(bevy_math::Vec3::new(1.0, -1.0, 1.0));
    let proj_matrix = flip_y * proj;
    unsafe {
        // Use the 'allocation' variable directly here, before it's moved
        let info = allocator.get_allocation_info(&allocation);
        if !info.mapped_data.is_null() {
            let data_ptr = info.mapped_data.cast::<f32>();
            data_ptr.copy_from_nonoverlapping(proj_matrix.to_cols_array().as_ptr(), 16);
        } else {
            error!("[create_global_ubo_system] Failed to get mapped pointer for initial uniform buffer write.");
        }
    }

    // 5. Insert Resource (Now move buffer, allocation, descriptor_set)
    commands.insert_resource(GlobalProjectionUboResource {
        buffer,
        allocation, // 'allocation' is moved here
        descriptor_set,
    });
}

fn create_glyph_atlas_system(
    mut commands: Commands,
    vk_context_res: Res<VulkanContextResource>,
) {
    let mut vk_ctx_guard = vk_context_res.0.lock().expect("Failed to lock VulkanContext for glyph atlas creation");

    // Choose an initial size for the atlas
    let initial_extent = vk::Extent2D { width: 1024, height: 1024 };

    match GlyphAtlas::new(&mut vk_ctx_guard, initial_extent) {
        Ok(atlas) => {
            let atlas_arc = Arc::new(Mutex::new(atlas));
            commands.insert_resource(GlyphAtlasResource(atlas_arc));
        }
        Err(e) => {
            // Use expect here because atlas is critical for text rendering
            panic!("Failed to create GlyphAtlas: {}", e);
        }
    }
}

fn create_font_server_system(mut commands: Commands) {
    // FontServer::new() can take some time if loading many system fonts.
    // Consider running this asynchronously or loading fewer fonts if startup time is critical.
    let font_server = FontServer::new();
    let font_server_arc = Arc::new(Mutex::new(font_server));
    commands.insert_resource(FontServerResource(font_server_arc));
}

fn create_text_rendering_resources_system(
    mut commands: Commands,
    vk_context_res: Res<VulkanContextResource>,
    renderer_res: Res<RendererResource>, // Need renderer for pool & atlas layout
    glyph_atlas_res: Res<GlyphAtlasResource>, // Need atlas for initial descriptor set update
) {
    info!("Running create_text_rendering_resources_system (Core Plugin)...");
    let Ok(vk_ctx_guard) = vk_context_res.0.lock() else {
        error!("Failed to lock VulkanContext in create_text_rendering_resources_system");
        return;
    };
     let Ok(renderer_guard) = renderer_res.0.lock() else {
        error!("Failed to lock RendererResource in create_text_rendering_resources_system");
        return;
    };
    let Ok(atlas_guard) = glyph_atlas_res.0.lock() else {
        error!("Failed to lock GlyphAtlasResource in create_text_rendering_resources_system");
        return;
    };
    let device = vk_ctx_guard.device.as_ref().expect("Device missing");
    let allocator = vk_ctx_guard.allocator.as_ref().expect("Allocator missing");

    // 1. Create Initial Dynamic Text Vertex Buffer (Shared)
    let initial_text_capacity = 1024 * 6; // Enough for ~1024 glyphs
    let buffer_size = (std::mem::size_of::<TextVertex>() * initial_text_capacity as usize) as vk::DeviceSize;
    let (vertex_buffer, vertex_allocation) = unsafe {
        let buffer_info = vk::BufferCreateInfo { 
            s_type: vk::StructureType::BUFFER_CREATE_INFO, 
            size: buffer_size, 
            usage: vk::BufferUsageFlags::VERTEX_BUFFER, 
            sharing_mode: vk::SharingMode::EXCLUSIVE, 
            ..Default::default() };
        let allocation_info = vk_mem::AllocationCreateInfo { 
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE 
            | vk_mem::AllocationCreateFlags::MAPPED,
            usage: vk_mem::MemoryUsage::AutoPreferDevice, 
            ..Default::default() };
        allocator.create_buffer(&buffer_info, &allocation_info).expect("Failed to create initial text vertex buffer")
    };
    // --- NAME Shared Text VB & Memory ---
    #[cfg(debug_assertions)]
    if let Some(debug_device_ext) = vk_ctx_guard.debug_utils_device.as_ref() { // Get Device ext
        let mem_handle = allocator.get_allocation_info(&vertex_allocation).device_memory;
        set_debug_object_name(debug_device_ext, vertex_buffer, vk::ObjectType::BUFFER, "SharedTextVertexBuffer"); // Pass ext
        set_debug_object_name(debug_device_ext, mem_handle, vk::ObjectType::DEVICE_MEMORY, "SharedTextVertexBuffer_Mem"); // Pass ext
    }
    // --- END NAME ---
    info!("[create_text_rendering_resources_system] Initial text vertex buffer created (Capacity: {} vertices, Size: {} bytes)", initial_text_capacity, buffer_size);
    // 2. Create Text Graphics Pipeline (Shared)
    let text_pipeline = unsafe {
        let render_pass = vk_ctx_guard.render_pass.expect("Render pass missing");
        let pipeline_layout = vk_ctx_guard.text_pipeline_layout.expect("Text pipeline layout missing"); // Uses per_entity_layout (Set 0) + atlas_layout (Set 1)
        let vert_shader_module = shader_utils::load_shader(device, "text.vert.spv");
        let frag_shader_module = shader_utils::load_shader(device, "text.frag.spv");
        // --- Define Pipeline Stages ---
        let shader_stages = [ vk::PipelineShaderStageCreateInfo { s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO, module: vert_shader_module, stage: vk::ShaderStageFlags::VERTEX, p_name: b"main\0".as_ptr() as _, ..Default::default() }, vk::PipelineShaderStageCreateInfo { s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO, module: frag_shader_module, stage: vk::ShaderStageFlags::FRAGMENT, p_name: b"main\0".as_ptr() as _, ..Default::default() }, ];
        // --- Define Vertex Input State ---
        let vertex_attr_descs = [ vk::VertexInputAttributeDescription { location: 0, binding: 0, format: vk::Format::R32G32_SFLOAT, offset: 0 }, vk::VertexInputAttributeDescription { location: 1, binding: 0, format: vk::Format::R32G32_SFLOAT, offset: std::mem::size_of::<[f32; 2]>() as u32 }, ];
        let vertex_binding_descs = [ vk::VertexInputBindingDescription { binding: 0, stride: std::mem::size_of::<TextVertex>() as u32, input_rate: vk::VertexInputRate::VERTEX } ];
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo { s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO, vertex_binding_description_count: vertex_binding_descs.len() as u32, p_vertex_binding_descriptions: vertex_binding_descs.as_ptr(), vertex_attribute_description_count: vertex_attr_descs.len() as u32, p_vertex_attribute_descriptions: vertex_attr_descs.as_ptr(), ..Default::default() };
        // --- Define Other Pipeline States ---
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo { s_type: vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO, topology: vk::PrimitiveTopology::TRIANGLE_LIST, ..Default::default() };
        let viewport_state = vk::PipelineViewportStateCreateInfo { s_type: vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO, viewport_count: 1, scissor_count: 1, ..Default::default() };
        let rasterizer = vk::PipelineRasterizationStateCreateInfo { s_type: vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO, polygon_mode: vk::PolygonMode::FILL, line_width: 1.0, cull_mode: vk::CullModeFlags::NONE, front_face: vk::FrontFace::CLOCKWISE, ..Default::default() };
        let multisampling = vk::PipelineMultisampleStateCreateInfo { s_type: vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO, rasterization_samples: vk::SampleCountFlags::TYPE_1, ..Default::default() };
        // Enable depth testing and writing for text to interact correctly with cursor/other elements
        let text_depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO,
            depth_test_enable: vk::TRUE,
            depth_write_enable: vk::TRUE,
            depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
            depth_bounds_test_enable: vk::FALSE,
            stencil_test_enable: vk::FALSE,
            ..Default::default()
        };
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState { blend_enable: vk::TRUE, src_color_blend_factor: vk::BlendFactor::SRC_ALPHA, dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA, color_blend_op: vk::BlendOp::ADD, src_alpha_blend_factor: vk::BlendFactor::ONE, dst_alpha_blend_factor: vk::BlendFactor::ZERO, alpha_blend_op: vk::BlendOp::ADD, color_write_mask: vk::ColorComponentFlags::RGBA, };
        let color_blending = vk::PipelineColorBlendStateCreateInfo { s_type: vk::StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO, logic_op_enable: vk::FALSE, attachment_count: 1, p_attachments: &color_blend_attachment, ..Default::default() };
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info = vk::PipelineDynamicStateCreateInfo { s_type: vk::StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO, dynamic_state_count: dynamic_states.len() as u32, p_dynamic_states: dynamic_states.as_ptr(), ..Default::default() };
        // --- Assemble Graphics Pipeline Create Info ---
        let pipeline_info = vk::GraphicsPipelineCreateInfo { s_type: vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO, stage_count: shader_stages.len() as u32, p_stages: shader_stages.as_ptr(), p_vertex_input_state: &vertex_input_info, p_input_assembly_state: &input_assembly, p_viewport_state: &viewport_state, p_rasterization_state: &rasterizer, p_multisample_state: &multisampling, p_depth_stencil_state: &text_depth_stencil_state, p_color_blend_state: &color_blending, p_dynamic_state: &dynamic_state_info, layout: pipeline_layout, render_pass, subpass: 0, ..Default::default() };
        // --- Create Pipeline ---
        let pipeline = device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None).expect("Failed to create text graphics pipeline").remove(0);
        // --- Cleanup Shader Modules ---
        device.destroy_shader_module(vert_shader_module, None);
        device.destroy_shader_module(frag_shader_module, None);
        pipeline // Return the created pipeline handle
    };
    info!("[create_text_rendering_resources_system] Text graphics pipeline created.");

    // 3. Allocate *Global* Glyph Atlas Descriptor Set (Set 1)
    // Use the atlas_layout stored in Renderer
    let set_layouts = [renderer_guard.text_descriptor_set_layout]; // This is now atlas_layout
    let alloc_info = vk::DescriptorSetAllocateInfo {
        s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
        descriptor_pool: renderer_guard.descriptor_pool,
        descriptor_set_count: 1,
        p_set_layouts: set_layouts.as_ptr(),
        ..Default::default()
    };
    let atlas_descriptor_set = unsafe {
        device.allocate_descriptor_sets(&alloc_info)
            .expect("Failed to allocate glyph atlas descriptor set")
            .remove(0)
    };
    info!("[create_text_rendering_resources_system] Glyph atlas descriptor set (Set 1) allocated.");

    // 4. Update Glyph Atlas Descriptor Set (Initial Binding)
    let image_info = vk::DescriptorImageInfo { sampler: atlas_guard.sampler, image_view: atlas_guard.image_view, image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL };
    let write_set = vk::WriteDescriptorSet { s_type: vk::StructureType::WRITE_DESCRIPTOR_SET, dst_set: atlas_descriptor_set, dst_binding: 0, dst_array_element: 0, descriptor_count: 1, descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER, p_image_info: &image_info, ..Default::default() };
    unsafe { device.update_descriptor_sets(&[write_set], &[]); }
    info!("[create_text_rendering_resources_system] Glyph atlas descriptor set (Set 1) updated.");

    // 5. Insert Resource (Vertex Buffer, Pipeline, Atlas Set)
    commands.insert_resource(TextRenderingResources {
        vertex_buffer,
        vertex_allocation,
        vertex_buffer_capacity: initial_text_capacity,
        pipeline: text_pipeline,
        atlas_descriptor_set, // Store the global atlas set here
    });
    info!("TextRenderingResources inserted (Core Plugin).");
}

// Update system: Handles window resize events, updates global UBO, and calls Renderer resize.
fn handle_resize_system(
    mut resize_reader: EventReader<bevy_window::WindowResized>,
    renderer_res_opt: Option<ResMut<RendererResource>>,
    // vk_context_res_opt: Option<Res<VulkanContextResource>>, // Removed Option<>
    global_ubo_res_opt: Option<Res<GlobalProjectionUboResource>>,
    vk_context_res: Res<VulkanContextResource>, // Get the resource directly
) {
    // Check if necessary resources are available (Renderer and Global UBO)
    let (Some(renderer_res), Some(global_ubo_res)) =
        (renderer_res_opt, global_ubo_res_opt) else { return; };

    for event in resize_reader.read() {
        if event.width > 0.0 && event.height > 0.0 {
            // --- Update Global Projection UBO ---
            let logical_width = event.width;
            let logical_height = event.height;
            // Get allocator via VulkanContext (Need to lock briefly just for this)
            // Use the vk_context_res passed directly into the system
            let allocator_opt = vk_context_res.0.lock().ok().and_then(|ctx| ctx.allocator.clone()); // Lock, get Arc, drop lock
            let Some(allocator) = allocator_opt else {
                 warn!("Could not get allocator from VulkanContext during handle_resize_system.");
                 continue; // Skip this event if allocator isn't ready
            };
            let proj = Mat4::orthographic_rh(0.0, logical_width, 0.0, logical_height, 1024.0, 0.0);
            let flip_y = Mat4::from_scale(bevy_math::Vec3::new(1.0, -1.0, 1.0));
            let proj_matrix = flip_y * proj;

            unsafe {
                // Use the allocation from the resource
                let info = allocator.get_allocation_info(&global_ubo_res.allocation);
                if !info.mapped_data.is_null() {
                    let data_ptr = info.mapped_data.cast::<f32>();
                    data_ptr.copy_from_nonoverlapping(proj_matrix.to_cols_array().as_ptr(), 16);
                    // Optional flush if not HOST_COHERENT
                    // if allocator.flush_allocation(&global_ubo_res.allocation, 0, vk::WHOLE_SIZE).is_err() {
                    //     error!("Failed to flush global UBO allocation on resize");
                    // }
                } else {
                    error!("[handle_resize_system] Failed to get mapped pointer for global UBO update.");
                }
            }

            // --- Call Renderer Resize ---
            // We no longer lock VulkanContext here. Renderer::resize_renderer will handle it.
            // We still need to lock the RendererResource to call the method.
            if let Ok(mut renderer_guard) = renderer_res.0.lock() {
                // Pass VulkanContextResource directly (using the system parameter vk_context_res)
                renderer_guard.resize_renderer(
                    &vk_context_res, // Pass the resource itself
                    event.width as u32,
                    event.height as u32,
                );
                // Renderer lock released when guard goes out of scope here
            } else {
                warn!("Could not lock RendererResource for renderer resize handling (Core Plugin).");
            }
        }
    }
}

/// System to spawn/despawn the visual cursor entity based on `Focus` component changes.
/// Adds/Removes `CursorState` from the focused entity.
fn manage_cursor_visual_system(
    mut commands: Commands,
    focus_added_query: Query<(Entity, &Transform), (Added<Focus>, Without<CursorVisual>)>,
    mut focus_removed_query: RemovedComponents<Focus>,
    mut cursor_visual_query: Query<(Entity, &Parent, &mut Visibility), With<CursorVisual>>,
    text_selection_query: Query<&TextSelection>,
    focused_query: Query<Entity, With<Focus>>,
    children_query: Query<&Children>,
) {
    // --- Handle Focus Gained ---
    for (focused_entity, _text_transform) in focus_added_query.iter() {
        info!("Focus gained by {:?}, spawning cursor visual.", focused_entity);

        // CursorState is already added by interaction_system, no need to add it here

        // Set initial visibility to true (cursor should be visible for collapsed selection)
        let initial_visibility = true;

        // Spawn the visual cursor entity as a child
        let cursor_entity = commands.spawn((
            CursorVisual,
            ShapeData {
                // Define a 1x1 unit quad centered at the origin.
                // It will be scaled and positioned dynamically by `update_cursor_transform_system`
                // using the Rect from cosmic-text's `layout_cursor` function.
                vertices: Arc::new(vec![
                    // Triangle 1
                    Vertex { position: [-0.5, -0.5] },
                    Vertex { position: [0.5, 0.5] },
                    Vertex { position: [0.5, -0.5] },
                    // Triangle 2
                    Vertex { position: [-0.5, -0.5] },
                    Vertex { position: [-0.5, 0.5] },
                    Vertex { position: [0.5, 0.5] },
                ]),
                color: Color::BLACK,
            },
            Transform::from_xyz(0.0, 0.0, -0.1),
            Visibility(initial_visibility),
            Interaction::default(),
            Name::new("CursorVisual"),
        )).id();

        commands.entity(focused_entity).add_child(cursor_entity);
    }

    // --- Update Visibility for Existing Cursors ---
    for focused_entity in focused_query.iter() {
        if let Ok(selection) = text_selection_query.get(focused_entity) {
            if let Ok(children) = children_query.get(focused_entity) {
                for &child in children.iter() {
                    if let Ok((_cursor_entity, _parent, mut visibility)) = cursor_visual_query.get_mut(child) {
                        visibility.0 = selection.start == selection.end;
                        break;
                    }
                }
            }
        }
    }

    // --- Handle Focus Lost ---
    for lost_focus_entity in focus_removed_query.read() {
        info!("Focus lost by {:?}, despawning cursor visual.", lost_focus_entity);

        // Remove CursorState
        commands.entity(lost_focus_entity).remove::<CursorState>();

        // Despawn CursorVisual
        for (cursor_entity, parent, _visibility) in cursor_visual_query.iter() {
            if parent.get() == lost_focus_entity {
                commands.entity(cursor_entity).despawn_recursive();
                break;
            }
        }
    }
}

/// System to update the visual cursor's position and size based on `CursorState` and `TextBufferCache`.
fn update_cursor_transform_system(
    focused_query: Query<(Entity, &CursorState, &TextBufferCache), With<Focus>>,
    mut cursor_visual_query: Query<&mut Transform, With<CursorVisual>>,
    children_query: Query<&Children>,
) {
    // There should only be one focused entity.
    if let Ok((focused_entity, cursor_state, text_cache)) = focused_query.get_single() {
        // The buffer cache must exist to calculate positions.
        if let Some(buffer) = text_cache.buffer.as_ref() {
            // Find the child entity that is the visual cursor.
            if let Ok(children) = children_query.get(focused_entity) {
                for &child in children.iter() {
                    if let Ok(mut cursor_transform) = cursor_visual_query.get_mut(child) {
                        // Find the layout run for the cursor's current line.
                        let run_opt = buffer.layout_runs().find(|run| run.line_i == cursor_state.line);

                        if let Some(run) = run_opt {
                            // --- NEW, SIMPLIFIED LOGIC ---
                            // Always calculate the visual position from the logical `cursor_state.position`.
                            // This ensures the visual cursor is always snapped to a valid character boundary.
                            let local_cursor = global_to_local_cursor(buffer, cursor_state.position);

                            // Find the glyph that the cursor is at the start of.
                            // The `find` method works perfectly for this.
                            let glyph_at_cursor = run.glyphs.iter().find(|g| g.start == local_cursor.index);

                            // The cursor's X position is the start of the glyph it's on.
                            // If no glyph is found (i.e., at the end of the line), use the line's end position.
                            let cursor_x_ydown = glyph_at_cursor.map_or_else(
                                || run.glyphs.first().map_or(0.0, |g| g.x) + run.line_w, // End of line
                                |g| g.x, // Start of glyph
                            );

                            // Vertical position and size logic remains the same.
                            let line_visual_height = run.line_height;
                            let cursor_width = 1.0;
                            let cursor_top_y_ydown = run.line_top;

                            // Calculate the center of the cursor rectangle.
                            let center_x_ydown = cursor_x_ydown + (cursor_width / 2.0);
                            let center_y_ydown = cursor_top_y_ydown + (line_visual_height / 2.0);

                            // Update the transform.
                            cursor_transform.translation.x = center_x_ydown;
                            cursor_transform.translation.y = -center_y_ydown;
                            cursor_transform.scale.x = cursor_width;
                            cursor_transform.scale.y = line_visual_height;

                        } else {
                            // If we can't find the layout run, hide the cursor.
                            cursor_transform.scale = bevy_math::Vec3::ZERO;
                        }
                        // We found the cursor and updated it, so we can break the inner loop.
                        break;
                    }
                }
            }
        } else {
            // If the text buffer isn't cached, hide the cursor.
            if let Ok(children) = children_query.get(focused_entity) {
                 for &child in children.iter() {
                    if let Ok(mut cursor_transform) = cursor_visual_query.get_mut(child) {
                        warn!("TextBufferCache.buffer not available for cursor positioning. Hiding cursor.");
                        cursor_transform.scale = bevy_math::Vec3::ZERO;
                        break;
                    }
                }
            }
        }
    }
}

fn create_swash_cache_system(mut commands: Commands) {
    let swash_cache = SwashCache::new(); // SwashCache::new() is available
    commands.insert_resource(SwashCacheResource(Mutex::new(swash_cache)));
}

fn text_layout_system(
    mut commands: Commands,
    mut event_reader: EventReader<YrsTextChanged>,
    text_component_query: Query<(&Text, &Transform, &Visibility)>,
    new_text_component_query: Query<Entity, Added<Text>>,
    mut text_buffer_cache_query: Query<&mut TextBufferCache>,
    yrs_doc_res: Res<YrsDocResource>,
    font_server_res: Res<FontServerResource>,
    glyph_atlas_res: Res<GlyphAtlasResource>,
    swash_cache_res: Res<SwashCacheResource>,
    vk_context_res: Res<VulkanContextResource>,
) {
    // Lock non-Vulkan resources first
    let Ok(mut font_server) = font_server_res.0.lock() else {
        error!("[text_layout_system] Failed to lock FontServerResource. Skipping text layout.");
        return;
    };
    let Ok(mut swash_cache) = swash_cache_res.0.lock() else {
        error!("[text_layout_system] Failed to lock SwashCacheResource. Skipping text layout.");
        return;
    };

    // Determine which entities need processing (copied from original)
    let mut entities_to_process: HashSet<Entity> = HashSet::new();
    for event in event_reader.read() { entities_to_process.insert(event.entity); }
    for entity in new_text_component_query.iter() { entities_to_process.insert(entity); }
    if entities_to_process.is_empty() { return; }

    // Now, if there are entities to process, get Vulkan handles.
    // This lock is held only to get the handles.
    let (device_clone, queue_opt, command_pool_opt, allocator_clone_opt) = {
        match vk_context_res.0.lock() {
            Ok(vk_guard) => (
                vk_guard.device.as_ref().map(|d| d.clone()),
                vk_guard.queue,
                vk_guard.command_pool,
                vk_guard.allocator.as_ref().map(|a| a.clone()),
            ),
            Err(_) => {
                error!("[text_layout_system] Failed to lock VulkanContextResource to get handles. Skipping text layout.");
                return;
            }
        }
    };

    // Ensure all necessary Vulkan handles were successfully retrieved
    let (Some(device), Some(queue), Some(command_pool), Some(allocator)) =
        (device_clone, queue_opt, command_pool_opt, allocator_clone_opt) else {
        error!("[text_layout_system] One or more essential Vulkan handles (device, queue, command_pool, allocator) are None. Skipping text layout.");
        return;
    };

    // Lock GlyphAtlas for the duration of processing entities that need glyphs
    let Ok(mut glyph_atlas) = glyph_atlas_res.0.lock() else {
        error!("[text_layout_system] Failed to lock GlyphAtlasResource. Skipping text layout.");
        return;
    };

    // --- Determine which entities need processing ---
    let mut entities_to_process: HashSet<Entity> = HashSet::new();

    // Add entities from events
    for event in event_reader.read() {
        entities_to_process.insert(event.entity);
    }

    // Add newly added entities
    for entity in new_text_component_query.iter() { // <-- Use renamed parameter
        entities_to_process.insert(entity);
    }

    if entities_to_process.is_empty() {
        return; // Nothing to do
    }

    // --- Loop through Entities with Text that has been updated ---
    for entity in entities_to_process {
        // Get the components for the specific entity
        let Ok((text, _transform, visibility)) = text_component_query.get(entity) else { // <-- Use renamed parameter
            warn!("[text_layout_system] Could not find components for entity {:?} signaled for update.", entity);
            continue;
        };

        if !visibility.is_visible() {
            continue;
        }

        // --- Get Text Content from YrsDocResource (using sync Transact) ---
        // Explicitly get the Arc<Mutex<HashMap>> before locking
        let text_map_arc: &Arc<Mutex<HashMap<Entity, TextRef>>> = &yrs_doc_res.text_map;
        let text_map_guard = text_map_arc.lock().expect("Failed to lock text_map mutex");
        let text_content = match text_map_guard.get(&entity) { // Call .get() on the MutexGuard
            Some(yrs_text_handle) => {
                // Access the Arc<Doc> within the resource and use synchronous Transact
                let txn = yrs_doc_res.doc.transact();
                yrs_text_handle.get_string(&txn)
            }
            None => {
                warn!("[text_layout_system] Entity {:?} has Text component but no corresponding YrsText in resource map. Skipping.", entity);
                continue;
            }
        };

        // --- Create Cosmic Text Buffer PER ENTITY being processed ---
        let metrics = Metrics::new(text.size, text.size * 1.2); // Use Metrics here
        let mut buffer = Buffer::new(&mut font_server.font_system, metrics); // Create buffer inside the loop

        // --- Set Text Content and Attributes ---
        let cosmic_color = match text.color {
             bevy_color::Color::Srgba(srgba) => CosmicColor::rgba((srgba.red * 255.0) as u8, (srgba.green * 255.0) as u8, (srgba.blue * 255.0) as u8, (srgba.alpha * 255.0) as u8),
             bevy_color::Color::LinearRgba(linear) => CosmicColor::rgba((linear.red * 255.0) as u8, (linear.green * 255.0) as u8, (linear.blue * 255.0) as u8, (linear.alpha * 255.0) as u8),
             bevy_color::Color::Hsla(hsla) => { let srgba: bevy_color::Srgba = hsla.into(); CosmicColor::rgba((srgba.red * 255.0) as u8, (srgba.green * 255.0) as u8, (srgba.blue * 255.0) as u8, (srgba.alpha * 255.0) as u8) },
             bevy_color::Color::Hsva(hsva) => { let srgba: bevy_color::Srgba = hsva.into(); CosmicColor::rgba((srgba.red * 255.0) as u8, (srgba.green * 255.0) as u8, (srgba.blue * 255.0) as u8, (srgba.alpha * 255.0) as u8) },
             bevy_color::Color::Hwba(hwba) => { let srgba: bevy_color::Srgba = hwba.into(); CosmicColor::rgba((srgba.red * 255.0) as u8, (srgba.green * 255.0) as u8, (srgba.blue * 255.0) as u8, (srgba.alpha * 255.0) as u8) },
             bevy_color::Color::Laba(laba) => { let srgba: bevy_color::Srgba = laba.into(); CosmicColor::rgba((srgba.red * 255.0) as u8, (srgba.green * 255.0) as u8, (srgba.blue * 255.0) as u8, (srgba.alpha * 255.0) as u8) },
             bevy_color::Color::Lcha(lcha) => { let srgba: bevy_color::Srgba = lcha.into(); CosmicColor::rgba((srgba.red * 255.0) as u8, (srgba.green * 255.0) as u8, (srgba.blue * 255.0) as u8, (srgba.alpha * 255.0) as u8) },
             bevy_color::Color::Oklaba(oklaba) => { let srgba: bevy_color::Srgba = oklaba.into(); CosmicColor::rgba((srgba.red * 255.0) as u8, (srgba.green * 255.0) as u8, (srgba.blue * 255.0) as u8, (srgba.alpha * 255.0) as u8) },
             bevy_color::Color::Oklcha(oklcha) => { let srgba: bevy_color::Srgba = oklcha.into(); CosmicColor::rgba((srgba.red * 255.0) as u8, (srgba.green * 255.0) as u8, (srgba.blue * 255.0) as u8, (srgba.alpha * 255.0) as u8) },
             bevy_color::Color::Xyza(xyza) => { let srgba: bevy_color::Srgba = xyza.into(); CosmicColor::rgba((srgba.red * 255.0) as u8, (srgba.green * 255.0) as u8, (srgba.blue * 255.0) as u8, (srgba.alpha * 255.0) as u8) },
        };
        let attrs = Attrs::new().color(cosmic_color);
        buffer.set_text(&mut font_server.font_system, &text_content, &attrs, Shaping::Advanced);

        // --- Set Wrapping ---
        if let Some(bounds) = text.bounds {
            buffer.set_size(&mut font_server.font_system, Some(bounds.x), Some(bounds.y));
            buffer.set_wrap(&mut font_server.font_system, Wrap::Word);
        } else {
            buffer.set_size(&mut font_server.font_system, None, None);
            buffer.set_wrap(&mut font_server.font_system, Wrap::None);
        }

        // --- Shape the Text ---
        buffer.shape_until_scroll(&mut font_server.font_system, true);

        // --- Prepare to collect glyphs for THIS entity ---
        let mut positioned_glyphs = Vec::new();

        // --- Loop through Layout Runs (Lines) ---
        for run in buffer.layout_runs() { // Use the buffer created inside the loop
            let baseline_y = -run.line_y;

            // --- Loop through Glyphs in the Run ---
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

                // Pass individual handles to add_glyph
                let add_result = glyph_atlas.add_glyph(
                    &device,
                    queue,
                    command_pool,
                    &allocator,
                    cache_key,
                    &swash_image
                );

                match add_result {
                    Ok(glyph_info_ref) => {
                        let glyph_info_copy = *glyph_info_ref;
                        let placement = swash_image.placement;
                        let width = placement.width as f32;
                        let height = placement.height as f32;

                        let font_arc: Arc<Font> = match font_server.font_system.get_font(layout_glyph.font_id) {
                            Some(f) => f,
                            None => { warn!("Font ID {:?} not found.", layout_glyph.font_id); continue; }
                        };

                        let swash_font_ref: FontRef = font_arc.as_swash();
                        let swash_metrics = swash_font_ref.metrics(&[]);
                        let units_per_em = swash_metrics.units_per_em as f32;

                        if units_per_em == 0.0 { warn!("Units per em is 0 for font ID {:?}.", layout_glyph.font_id); continue; }

                        let relative_left_x = layout_glyph.x;
                        let relative_right_x = relative_left_x + width;
                        let relative_top_y = baseline_y + placement.top as f32;
                        let relative_bottom_y = relative_top_y - height;

                        let top_left = Vec2::new(relative_left_x, relative_top_y);
                        let top_right = Vec2::new(relative_right_x, relative_top_y);
                        let bottom_right = Vec2::new(relative_right_x, relative_bottom_y);
                        let bottom_left = Vec2::new(relative_left_x, relative_bottom_y);

                        let relative_vertices = [top_left, top_right, bottom_right, bottom_left];

                        positioned_glyphs.push(PositionedGlyph {
                            glyph_info: glyph_info_copy,
                            layout_glyph: layout_glyph.clone(),
                            vertices: relative_vertices,
                        });
                    }
                    Err(e) => { warn!("Failed to add glyph to atlas: {:?}", e); }
                }
            }
        }

        // --- Update/Insert Components AFTER processing all glyphs for the entity ---
        // 1. Update/Insert TextLayoutOutput
        commands.entity(entity).insert(TextLayoutOutput {
            glyphs: positioned_glyphs,
        });

        // 2. Update/Insert TextBufferCache
        let buffer_line_count = buffer.lines.len(); // Get line count before moving buffer
        if let Ok(mut cache) = text_buffer_cache_query.get_mut(entity) {
            info!("[text_layout_system] Updating TextBufferCache for {:?} ({} lines)", entity, buffer_line_count);
            cache.buffer = Some(buffer);
        } else {
            info!("[text_layout_system] Inserting TextBufferCache for {:?} ({} lines)", entity, buffer_line_count);
            commands.entity(entity).insert(TextBufferCache {
                buffer: Some(buffer),
            });
        }
    }
}

// Define a helper struct to pass text layout info to the renderer
#[derive(Clone)]
pub struct TextLayoutInfo {
    pub entity: Entity, // Make fields public
    pub transform: GlobalTransform,
    pub layout: Arc<TextLayoutOutput>,
    pub visibility: Visibility,
}

fn rendering_system(
    // Resources needed for rendering
    renderer_res_opt: Option<ResMut<RendererResource>>,
    vk_context_res_opt: Option<Res<VulkanContextResource>>,
    buffer_manager_res_opt: Option<Res<BufferManagerResource>>,
    global_ubo_res_opt: Option<Res<GlobalProjectionUboResource>>,
    text_res_opt: Option<Res<TextRenderingResources>>, // Still need pipeline/atlas set

    // Queries for scene data
    shape_query: Query<(Entity, &GlobalTransform, &ShapeData, &Visibility), (Without<TextLayoutOutput>, Or<(With<ShapeData>, With<CursorVisual>)>)>, // Query shapes/cursors without TextLayoutOutput
    shape_change_query: Query<Entity, (With<Visibility>, Changed<ShapeData>)>,
    // Query for text entities that have layout output ready
    text_layout_query: Query<(Entity, &GlobalTransform, &TextLayoutOutput, &Visibility)>, // Query layout output
) {
    // Ensure essential non-text resources are available
    let (
        Some(renderer_res),
        Some(vk_context_res),
        Some(buffer_manager_res),
        Some(global_ubo_res),
    ) = (renderer_res_opt, vk_context_res_opt, buffer_manager_res_opt, global_ubo_res_opt) else {
        warn!("[rendering_system] Required non-text resources not available. Skipping render.");
        return;
    };

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
                color: shape.color, // Get color from ShapeData
                depth: global_transform.translation().z,
                vertices_changed,
            });
        }
    }
    // Sort shapes by depth (optional, but good practice)
    shape_render_commands.sort_unstable_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap_or(std::cmp::Ordering::Equal));

    // --- Collect Text Layout Info ---
    // Collect layout data for visible text entities. Renderer will handle resource creation/update.
    let mut text_layout_infos: Vec<TextLayoutInfo> = Vec::new();
    for (entity, transform, layout_output, visibility) in text_layout_query.iter() {
        if visibility.is_visible() {
            text_layout_infos.push(TextLayoutInfo {
                entity,
                transform: *transform, // Copy GlobalTransform
                layout: Arc::new(layout_output.clone()), // Clone layout into an Arc
                visibility: *visibility, // Copy Visibility
            });
        }
    }
    // TODO: Sort text layouts by depth if needed (using transform.translation().z)

    // --- Call Custom Renderer ---
    let renderer_guard_opt = renderer_res.0.lock().ok(); // Bind Option<Guard> to variable first
    if let Some(mut renderer_guard) = renderer_guard_opt {
        renderer_guard.render(
            &vk_context_res,
            &buffer_manager_res,
            &shape_render_commands,
            &text_layout_infos, // Still empty if text_layout_system disabled
            &global_ubo_res,
            text_res_opt.as_deref(), // Pass Option<&TextRenderingResources>
        );
        // Guard dropped here
    } else {
        warn!("Could not lock RendererResource for rendering trigger (Core Plugin).");
    }
}

// System running on AppExit in Last schedule: Takes ownership of all Vulkan-related resources
// from the World and orchestrates their destruction in the correct order.
fn cleanup_trigger_system(world: &mut World) {
    info!("[Cleanup] Cleanup trigger system running on AppExit...");

    // --- Take ownership of all resources by removing them from the world ---
    let renderer_res_opt = world.remove_resource::<RendererResource>();
    let buffer_manager_res_opt = world.remove_resource::<BufferManagerResource>();
    let text_rendering_res_opt = world.remove_resource::<TextRenderingResources>();
    let global_ubo_res_opt = world.remove_resource::<GlobalProjectionUboResource>();
    let glyph_atlas_res_opt = world.remove_resource::<GlyphAtlasResource>();
    world.remove_resource::<crate::PreparedTextDrawsResource>();

    // The VulkanContextResource is the last one we take, as it contains the core handles.
    let vk_context_res_opt = world.remove_resource::<VulkanContextResource>();

    // --- Deconstruct VulkanContext ---
    // If the context doesn't exist, we can't clean anything up.
    let Some(vk_context_res) = vk_context_res_opt else {
        warn!("[Cleanup] VulkanContext resource not found. Cannot perform Vulkan cleanup.");
        return;
    };

    // Move the context out of the Mutex. This is the only time it's safe to do so.
    let mut vk_context = match vk_context_res.0.lock() {
        Ok(mut guard) => std::mem::replace(&mut *guard, VulkanContext::new()),
        Err(_) => {
            error!("[Cleanup] VulkanContext Mutex was poisoned. Cleanup cannot proceed safely.");
            return;
        }
    };

    // Extract the core device and allocator handles. These will be passed to other cleanup functions.
    let (Some(device), Some(allocator)) = (vk_context.device.as_ref(), vk_context.allocator.as_ref()) else {
        error!("[Cleanup] Vulkan Device or Allocator handle not found in context. Aborting cleanup.");
        return;
    };

    // --- Explicitly Wait for GPU to be Idle ---
    info!("[Cleanup] Waiting for device idle before resource destruction...");
    unsafe {
        if let Err(e) = device.device_wait_idle() {
            error!("[Cleanup] Failed to wait for device idle: {:?}", e);
        }
    }

    // --- Cleanup Dependent Resources (in reverse order of creation) ---

    if let Some(bm_res) = buffer_manager_res_opt {
        if let Ok(mut bm_guard) = bm_res.0.lock() {
            bm_guard.cleanup(device, allocator);
        }
    }

    if let Some(renderer_res) = renderer_res_opt {
        if let Ok(mut renderer_guard) = renderer_res.0.lock() {
            renderer_guard.cleanup(device, allocator);
        }
    }

    if let Some(mut text_res) = text_rendering_res_opt {
        info!("[Cleanup] Cleaning up TextRenderingResources...");
        unsafe {
            device.destroy_pipeline(text_res.pipeline, None);
            allocator.destroy_buffer(text_res.vertex_buffer, &mut text_res.vertex_allocation);
        }
    }

    if let Some(mut global_ubo_res) = global_ubo_res_opt {
        info!("[Cleanup] Cleaning up GlobalProjectionUboResource...");
        unsafe {
            allocator.destroy_buffer(global_ubo_res.buffer, &mut global_ubo_res.allocation);
        }
    }

    if let Some(atlas_res) = glyph_atlas_res_opt {
        if let Ok(mut atlas_guard) = atlas_res.0.lock() {
            atlas_guard.cleanup(device, allocator);
        }
    }

    // --- Final Swapchain and Context Cleanup ---
    // This function now takes the mutable context directly.
    crate::gui_framework::rendering::swapchain::cleanup_swapchain_resources(&mut vk_context);

    // Now, call the final cleanup function with the deconstructed context.
    cleanup_vulkan(vk_context);

    info!("[Cleanup] Cleanup trigger system finished.");
}