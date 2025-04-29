use bevy_app::{App, AppExit, Plugin, Startup, Update, Last};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{SystemSet, common_conditions::{not, on_event}};
use bevy_log::{info, error, warn};
use bevy_window::{PrimaryWindow, Window};
use bevy_winit::WinitWindows;
use bevy_transform::prelude::{GlobalTransform, Transform};
use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use ash::vk;
use bevy_color::Color;
use bevy_math::{Vec2, IVec2, Mat4};
use cosmic_text::{Attrs, BufferLine, FontSystem, Metrics, Shaping, SwashCache, Wrap, Color as CosmicColor};
use vk_mem::Alloc;

// Import types from the crate root (lib.rs)
use crate::{
    Vertex, RenderCommandData, TextVertex,
    PreparedTextDrawData,
    GlobalProjectionUboResource,
    TextRenderingResources, // text Vulkan objects
};

// Import types/functions from the gui_framework
use crate::gui_framework::{
    context::vulkan_setup::{setup_vulkan, cleanup_vulkan},
    rendering::render_engine::Renderer,
    rendering::glyph_atlas::{GlyphAtlas, GlyphInfo},
    rendering::font_server::FontServer,
    components::{ShapeData, Visibility, Text, FontId, TextAlignment, TextLayoutOutput, PositionedGlyph},
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
    HandleResize,           // Handle window resize events, update global UBO
    TextLayout,             // Perform text layout using cosmic-text
    PrepareTextRendering,   // Prepare text vertex data and Vulkan resources

    // Last sequence
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
                CoreSet::CreateRenderer,        // Depends on SetupVulkan
                CoreSet::CreateGlyphAtlas,      // Depends on SetupVulkan
                CoreSet::CreateFontServer,      
                CoreSet::CreateSwashCache,      
            ).chain())

            // Configure sets that have specific dependencies *after* the chain is defined
            .configure_sets(Startup, (
                // CreateGlobalUbo must run after CreateRenderer because it needs RendererResource
                CoreSet::CreateGlobalUbo.after(CoreSet::CreateRenderer),
                // CreateTextResources must run after CreateRenderer (needs layouts/pool)
                // and after CreateGlyphAtlas (needs atlas resource for descriptor set)
                CoreSet::CreateTextResources
                    .after(CoreSet::CreateRenderer)
                    .after(CoreSet::CreateGlyphAtlas),
        ))
            // Add the systems to the Startup schedule and assign them to their respective sets.
            // Rely on the configure_sets calls above to establish the execution order.
            .add_systems(Startup, (
                setup_vulkan_system.in_set(CoreSet::SetupVulkan),
                create_renderer_system.in_set(CoreSet::CreateRenderer),
                create_glyph_atlas_system.in_set(CoreSet::CreateGlyphAtlas),
                create_font_server_system.in_set(CoreSet::CreateFontServer),
                create_swash_cache_system.in_set(CoreSet::CreateSwashCache),
                create_global_ubo_system.in_set(CoreSet::CreateGlobalUbo),
                create_text_rendering_resources_system.in_set(CoreSet::CreateTextResources),
            )) 
            // Initialize the PreparedTextDrawsResource
            .init_resource::<crate::PreparedTextDrawsResource>()

            // == Update Systems ==
            // Define the desired execution order for Update systems
            .configure_sets(Update, (
                CoreSet::TextLayout, // Run text layout first
                CoreSet::PrepareTextRendering.after(CoreSet::TextLayout), // Then prepare text rendering
                CoreSet::HandleResize.after(CoreSet::PrepareTextRendering), // Finally, handle resize *after* text prep
        ).chain()) // Chain these sets to enforce the order
            .add_systems(Update, (
                handle_resize_system.in_set(CoreSet::HandleResize),
                text_layout_system.in_set(CoreSet::TextLayout),
                prepare_text_rendering_system.in_set(CoreSet::PrepareTextRendering),
        ))
            .configure_sets(Last, (
                // Ensure Render runs after TextLayout and PrepareTextRendering
                CoreSet::Render.after(CoreSet::TextLayout).after(CoreSet::PrepareTextRendering),
                CoreSet::Cleanup.after(CoreSet::Render), // Ensure cleanup runs last
        ))
            // == Rendering System (runs late) ==
            .add_systems(Last, (
                rendering_system.run_if(not(on_event::<AppExit>)).in_set(CoreSet::Render),
                cleanup_trigger_system.run_if(on_event::<AppExit>).in_set(CoreSet::Cleanup).after(CoreSet::Render),
        ));
    }
}


// --- Systems Moved from main.rs ---

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

    let renderer_instance = Renderer::new(&mut vk_ctx_guard, extent);

    let renderer_arc = Arc::new(Mutex::new(renderer_instance));
    commands.insert_resource(RendererResource(renderer_arc.clone()));
}

fn create_global_ubo_system(
    mut commands: Commands,
    vk_context_res: Res<VulkanContextResource>,
    renderer_res: Res<RendererResource>, // Need renderer for descriptor pool access
    primary_window_q: Query<&Window, With<PrimaryWindow>>, // Add window query parameter
) {
    info!("Running create_global_ubo_system (Core Plugin)...");
    let Ok(mut vk_ctx_guard) = vk_context_res.0.lock() else {
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
            flags: vk_mem::AllocationCreateFlags::MAPPED | vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            ..Default::default()
        };
        allocator.create_buffer(&buffer_info, &allocation_info)
                 .expect("Global UBO buffer creation failed")
    };

    // 2. Allocate Descriptor Set
    let set_layouts = [renderer_guard.descriptor_set_layout]; // Shape layout
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
    renderer_res: Res<RendererResource>, // Need renderer for layouts/pool
    glyph_atlas_res: Res<GlyphAtlasResource>, // Need atlas for initial descriptor set update
) {
    let Ok(mut vk_ctx_guard) = vk_context_res.0.lock() else {
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

    // 1. Create Initial Dynamic Text Vertex Buffer
    let initial_text_capacity = 1024 * 6; // Enough for ~1024 glyphs
    let buffer_size = (std::mem::size_of::<TextVertex>() * initial_text_capacity as usize) as vk::DeviceSize;
    let (vertex_buffer, vertex_allocation) = unsafe {
        let buffer_info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            size: buffer_size,
            usage: vk::BufferUsageFlags::VERTEX_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let allocation_info = vk_mem::AllocationCreateInfo {
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE | vk_mem::AllocationCreateFlags::MAPPED,
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };
        allocator.create_buffer(&buffer_info, &allocation_info)
                 .expect("Failed to create initial text vertex buffer")
    };

    // 2. Create Text Graphics Pipeline
    let text_pipeline = unsafe {
        let render_pass = vk_ctx_guard.render_pass.expect("Render pass missing");
        // Use the text pipeline layout stored in VulkanContext (set by Renderer::new)
        let pipeline_layout = vk_ctx_guard.text_pipeline_layout.expect("Text pipeline layout missing");

        // Load shaders
        let vert_shader_module = shader_utils::load_shader(device, "text.vert.spv");
        let frag_shader_module = shader_utils::load_shader(device, "text.frag.spv");

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo { s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO, module: vert_shader_module, stage: vk::ShaderStageFlags::VERTEX, p_name: b"main\0".as_ptr() as _, ..Default::default() },
            vk::PipelineShaderStageCreateInfo { s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO, module: frag_shader_module, stage: vk::ShaderStageFlags::FRAGMENT, p_name: b"main\0".as_ptr() as _, ..Default::default() },
        ];
        let vertex_attr_descs = [
            vk::VertexInputAttributeDescription { location: 0, binding: 0, format: vk::Format::R32G32_SFLOAT, offset: 0 },
            vk::VertexInputAttributeDescription { location: 1, binding: 0, format: vk::Format::R32G32_SFLOAT, offset: std::mem::size_of::<[f32; 2]>() as u32 },
        ];
        let vertex_binding_descs = [
            vk::VertexInputBindingDescription { binding: 0, stride: std::mem::size_of::<TextVertex>() as u32, input_rate: vk::VertexInputRate::VERTEX }
        ];
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
            vertex_binding_description_count: vertex_binding_descs.len() as u32,
            p_vertex_binding_descriptions: vertex_binding_descs.as_ptr(),
            vertex_attribute_description_count: vertex_attr_descs.len() as u32,
            p_vertex_attribute_descriptions: vertex_attr_descs.as_ptr(),
            ..Default::default()
        };

        // --- Define Other Pipeline States ---
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            ..Default::default()
        };
        let viewport_state = vk::PipelineViewportStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
            viewport_count: 1, // Dynamic state
            scissor_count: 1,  // Dynamic state
            ..Default::default()
        };
        let rasterizer = vk::PipelineRasterizationStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
            polygon_mode: vk::PolygonMode::FILL,
            line_width: 1.0,
            cull_mode: vk::CullModeFlags::NONE, // No culling for 2D text quads
            front_face: vk::FrontFace::CLOCKWISE, // Match vertex order
            ..Default::default()
        };
        let multisampling = vk::PipelineMultisampleStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
            rasterization_samples: vk::SampleCountFlags::TYPE_1, // No MSAA
            ..Default::default()
        };
        // Define depth state for text (test/write disabled, but state struct needed)
        let text_depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO,
            depth_test_enable: vk::FALSE,
            depth_write_enable: vk::FALSE,
            depth_compare_op: vk::CompareOp::LESS_OR_EQUAL, // Doesn't matter if test disabled
            ..Default::default()
        };
        // Define color blending for transparency
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
            blend_enable: vk::TRUE,
            src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
            dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ONE, // Use source alpha directly
            dst_alpha_blend_factor: vk::BlendFactor::ZERO, // Don't blend alpha with destination
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::RGBA,
        };
        let color_blending = vk::PipelineColorBlendStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
            logic_op_enable: vk::FALSE,
            attachment_count: 1,
            p_attachments: &color_blend_attachment,
            ..Default::default()
        };
        // Define dynamic states
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info = vk::PipelineDynamicStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO,
            dynamic_state_count: dynamic_states.len() as u32,
            p_dynamic_states: dynamic_states.as_ptr(),
            ..Default::default()
        };

        // --- Assemble Graphics Pipeline Create Info ---
        let pipeline_info = vk::GraphicsPipelineCreateInfo {
            s_type: vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
            stage_count: shader_stages.len() as u32,
            p_stages: shader_stages.as_ptr(),
            p_vertex_input_state: &vertex_input_info,
            p_input_assembly_state: &input_assembly,
            p_viewport_state: &viewport_state,
            p_rasterization_state: &rasterizer,
            p_multisample_state: &multisampling,
            p_depth_stencil_state: &text_depth_stencil_state, // Assign depth state
            p_color_blend_state: &color_blending,
            p_dynamic_state: &dynamic_state_info,
            layout: pipeline_layout,
            render_pass,
            subpass: 0,
            ..Default::default()
        };
        let pipeline = device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
            .expect("Failed to create text graphics pipeline")
            .remove(0);
        device.destroy_shader_module(vert_shader_module, None);
        device.destroy_shader_module(frag_shader_module, None);
        pipeline
    };

    // 3. Allocate Glyph Atlas Descriptor Set
    // Use the *text* descriptor set layout (binding 0 is atlas sampler)
    let set_layouts = [renderer_guard.text_descriptor_set_layout]; // Text layout
    let alloc_info = vk::DescriptorSetAllocateInfo {
        s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
        descriptor_pool: renderer_guard.descriptor_pool, // Access pool field
        descriptor_set_count: 1,
        p_set_layouts: set_layouts.as_ptr(),
        ..Default::default()
    };
    let atlas_descriptor_set = unsafe {
        device.allocate_descriptor_sets(&alloc_info)
            .expect("Failed to allocate glyph atlas descriptor set")
            .remove(0)
    };

    // 4. Update Glyph Atlas Descriptor Set (Initial Binding)
    let image_info = vk::DescriptorImageInfo {
        sampler: atlas_guard.sampler,
        image_view: atlas_guard.image_view,
        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, // Assume layout is correct after upload
    };
    let write_set = vk::WriteDescriptorSet {
        s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
        dst_set: atlas_descriptor_set,
        dst_binding: 0, // Binding 0 for atlas sampler in text layout
        dst_array_element: 0,
        descriptor_count: 1,
        descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
        p_image_info: &image_info,
        ..Default::default()
    };
    unsafe { device.update_descriptor_sets(&[write_set], &[]); }

    // 5. Insert Resource
    commands.insert_resource(TextRenderingResources {
        vertex_buffer,
        vertex_allocation,
        vertex_buffer_capacity: initial_text_capacity,
        pipeline: text_pipeline,
        atlas_descriptor_set,
    });
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
                    &global_ubo_res
                );
                // Renderer lock released when guard goes out of scope here
            } else {
                warn!("Could not lock RendererResource for renderer resize handling (Core Plugin).");
            }
        }
    }
}

// Update system: Prepares text rendering data (vertices, draw commands) for changed text entities.
fn prepare_text_rendering_system(
    mut commands: Commands,
    // Query for text entities whose layout has changed
    query: Query<(Entity, &GlobalTransform, &TextLayoutOutput, &Visibility)>,
    // Access the text rendering Vulkan resources
    mut text_res_opt: Option<ResMut<TextRenderingResources>>,
    // Access the global projection UBO resource
    global_ubo_res_opt: Option<Res<GlobalProjectionUboResource>>,
    // Access Vulkan Context to get allocator
    vk_context_res_opt: Option<Res<VulkanContextResource>>,
    // Get the resource to store prepared draw data
    mut prepared_text_draws_res: ResMut<crate::PreparedTextDrawsResource>,
) {
    // Get resources, including allocator via vk_context
    let Some(mut text_res) = text_res_opt else {
        warn!("[prepare_text] TextRenderingResources not found."); return;
    };
    let Some(global_ubo_res) = global_ubo_res_opt else {
         warn!("[prepare_text] GlobalProjectionUboResource not found."); return;
    };
    let Some(vk_context_res) = vk_context_res_opt else { return; };

    let Ok(vk_ctx_guard) = vk_context_res.0.lock() else {
        warn!("Could not lock VulkanContext in prepare_text_rendering_system.");
        return;
    };
    let Some(allocator) = vk_ctx_guard.allocator.as_ref() else {
        warn!("Allocator not found in VulkanContext during prepare_text_rendering_system.");
        return;
    };
    // Drop the guard quickly as we only needed the allocator Arc clone
    // Or keep it if other vk_context fields are needed later
    // let allocator = allocator.clone(); // Clone Arc if guard needs to be dropped
    // drop(vk_ctx_guard);

    // Clear previous frame's prepared data from the resource
    prepared_text_draws_res.0.clear();

    // --- Collect Vertices for Changed Text ---
    // This is the simplified approach: collect all vertices if *any* text changed.
    // A more optimized approach would track offsets and update sub-regions.
    let mut all_text_vertices: Vec<TextVertex> = Vec::new();
    let mut needs_buffer_update = false;

    for (entity, global_transform, text_layout, visibility) in query.iter() { // Added entity to log
        if visibility.is_visible() {
            needs_buffer_update = true; // Mark that an update is needed
            let transform_matrix = global_transform.compute_matrix();
            for positioned_glyph in &text_layout.glyphs {
                let world_pos = |rel_pos: Vec2| {
                    transform_matrix.transform_point3(rel_pos.extend(0.0)).truncate()
                };
                let tl_world = world_pos(positioned_glyph.vertices[0]);
                let tr_world = world_pos(positioned_glyph.vertices[1]);
                let br_world = world_pos(positioned_glyph.vertices[2]);
                let bl_world = world_pos(positioned_glyph.vertices[3]);
                let uv_min = positioned_glyph.glyph_info.uv_min;
                let uv_max = positioned_glyph.glyph_info.uv_max;

                // Triangle 1
                all_text_vertices.push(TextVertex { position: tl_world.into(), uv: [uv_min[0], uv_min[1]] });
                all_text_vertices.push(TextVertex { position: bl_world.into(), uv: [uv_min[0], uv_max[1]] });
                all_text_vertices.push(TextVertex { position: br_world.into(), uv: [uv_max[0], uv_max[1]] });
                // Triangle 2
                all_text_vertices.push(TextVertex { position: tl_world.into(), uv: [uv_min[0], uv_min[1]] });
                all_text_vertices.push(TextVertex { position: br_world.into(), uv: [uv_max[0], uv_max[1]] });
                all_text_vertices.push(TextVertex { position: tr_world.into(), uv: [uv_max[0], uv_min[1]] });
            }
        }
    }

    // --- Update Vulkan Vertex Buffer ---
    if needs_buffer_update {
        let num_text_vertices = all_text_vertices.len() as u32;
        if num_text_vertices > 0 {
            // Check if buffer needs resizing
            if num_text_vertices > text_res.vertex_buffer_capacity {
                let new_capacity = (num_text_vertices * 2).max(text_res.vertex_buffer_capacity * 2);
                let new_size = (std::mem::size_of::<TextVertex>() * new_capacity as usize) as vk::DeviceSize;
                // Destroy old buffer/allocation
                unsafe { allocator.destroy_buffer(text_res.vertex_buffer, &mut text_res.vertex_allocation); }
                // Create new buffer/allocation
                let (new_buffer, new_alloc) = unsafe {
                    let buffer_info = vk::BufferCreateInfo { s_type: vk::StructureType::BUFFER_CREATE_INFO, size: new_size, usage: vk::BufferUsageFlags::VERTEX_BUFFER, sharing_mode: vk::SharingMode::EXCLUSIVE, ..Default::default() };
                    let allocation_info = vk_mem::AllocationCreateInfo { flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE | vk_mem::AllocationCreateFlags::MAPPED, usage: vk_mem::MemoryUsage::AutoPreferDevice, ..Default::default() };
                    allocator.create_buffer(&buffer_info, &allocation_info).expect("Failed to resize text vertex buffer")
                };
                text_res.vertex_buffer = new_buffer;
                text_res.vertex_allocation = new_alloc;
                text_res.vertex_buffer_capacity = new_capacity;
            }

            // Copy data to text vertex buffer
            unsafe {
                let info = allocator.get_allocation_info(&text_res.vertex_allocation);
                if !info.mapped_data.is_null() {
                    let data_ptr = info.mapped_data.cast::<TextVertex>();
                    data_ptr.copy_from_nonoverlapping(all_text_vertices.as_ptr(), num_text_vertices as usize);
                    // Optional flush
                    // allocator.flush_allocation(&text_res.vertex_allocation, 0, vk::WHOLE_SIZE).expect("Failed to flush text vertex buffer");
                } else {
                    error!("[prepare_text_rendering_system] Text vertex buffer allocation not mapped during update!");
                }
            }

            // --- Create PreparedTextDrawData and add it to the resource ---
            // For now, create one draw command for all updated text
            prepared_text_draws_res.0.push(PreparedTextDrawData { // Add to resource's Vec
                pipeline: text_res.pipeline,
                vertex_buffer: text_res.vertex_buffer,
                vertex_buffer_offset: 0, // Start from beginning
                vertex_count: num_text_vertices,
                projection_descriptor_set: global_ubo_res.descriptor_set, // Use global UBO set
                atlas_descriptor_set: text_res.atlas_descriptor_set, // Use text resource's atlas set
            });
        } else {
             info!("[prepare_text] No text vertices generated, skipping buffer update and draw data creation.");
        }
    }
    // Store the prepared draws in Commands as a NonSend resource for the rendering system?
    // Or maybe the rendering system can just query this Local? Let's try Local first.
    // If this becomes complex, use a dedicated resource.
}

fn create_swash_cache_system(mut commands: Commands) {
    let swash_cache = SwashCache::new(); // SwashCache::new() is available
    commands.insert_resource(SwashCacheResource(Mutex::new(swash_cache)));
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

                let add_result = glyph_atlas.add_glyph(&vk_context, cache_key, &swash_image);

                match add_result {
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
    global_ubo_res_opt: Option<Res<GlobalProjectionUboResource>>, // Need global UBO for shapes
    // Query for shapes
    shape_query: Query<(Entity, &GlobalTransform, &ShapeData, &Visibility), Without<TextLayoutOutput>>,
    shape_change_query: Query<Entity, (With<Visibility>, Changed<ShapeData>)>,
    // Access the prepared text draw data via the resource
    prepared_text_draws_res: Res<crate::PreparedTextDrawsResource>,
) {
    // Ensure all required resources are available
    if let (Some(renderer_res), Some(vk_context_res), Some(global_ubo_res)) =
        (renderer_res_opt, vk_context_res_opt, global_ubo_res_opt)
    {
        // --- Collect Shape Render Data (Unchanged) ---
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

        // --- Call Custom Renderer ---
        // Lock only the Renderer resource here. Renderer::render will handle VulkanContext locking.
        if let Ok(mut renderer_guard) = renderer_res.0.lock() {
            // Pass the VulkanContextResource directly
            renderer_guard.render(
                &vk_context_res, // <-- Pass the resource
                &shape_render_commands,
                &prepared_text_draws_res.0,
                &global_ubo_res,
            );
        } else {
            warn!("Could not lock RendererResource for rendering trigger (Core Plugin).");
        }
    }
}


// System running on AppExit in Last schedule: Takes ownership of Vulkan/Renderer resources via World access and cleans them up immediately.
fn cleanup_trigger_system(world: &mut World) {
    // Take ownership of resources by removing them from the world
    let renderer_res_opt: Option<RendererResource> = world.remove_resource::<RendererResource>();
    let vk_context_res_opt: Option<VulkanContextResource> = world.remove_resource::<VulkanContextResource>();
    let glyph_atlas_res_opt: Option<GlyphAtlasResource> = world.remove_resource::<GlyphAtlasResource>();
    let text_rendering_res_opt: Option<TextRenderingResources> = world.remove_resource::<TextRenderingResources>();
    let global_ubo_res_opt: Option<GlobalProjectionUboResource> = world.remove_resource::<GlobalProjectionUboResource>();
    let _prepared_text_draws_res_opt: Option<crate::PreparedTextDrawsResource> = world.remove_resource::<crate::PreparedTextDrawsResource>(); // Remove text draw resource
    // FontServer and SwashCache don't need Vulkan cleanup

    if let Some(vk_context_res) = vk_context_res_opt {
        info!("VulkanContextResource taken (Core Plugin).");
        match vk_context_res.0.lock() {
            Ok(mut vk_ctx_guard) => {
                info!("Successfully locked VulkanContext Mutex (Core Plugin).");

                // --- Cleanup Order ---
                // 1. Cleanup Renderer (Needs mutable vk_ctx_guard to destroy its owned Vulkan objects like pool, layouts, sync objects)
                if let Some(renderer_res) = renderer_res_opt {
                    info!("RendererResource taken (Core Plugin).");
                    match renderer_res.0.lock() {
                        Ok(mut renderer_guard) => {
                            info!("Successfully locked Renderer Mutex (Core Plugin).");
                            // Assuming Renderer::cleanup needs &mut VulkanContext
                            renderer_guard.cleanup(&mut vk_ctx_guard);
                        }
                        Err(poisoned) => error!("Renderer Mutex poisoned: {:?}", poisoned),
                    }
                } else { info!("Renderer resource not found (Core Plugin)."); }

                // Now that Renderer is cleaned up (and its mutable borrow of vk_ctx_guard is released),
                // we can safely get immutable references to device/allocator if needed for other cleanup.
                let device_opt = vk_ctx_guard.device.as_ref();
                let allocator_opt = vk_ctx_guard.allocator.as_ref();

                if let (Some(device), Some(allocator)) = (device_opt, allocator_opt) {

                    // 2. Cleanup TextRenderingResources (Buffer, Pipeline)
                    if let Some(mut text_res) = text_rendering_res_opt {
                        unsafe {
                            if text_res.pipeline != vk::Pipeline::null() {
                                device.destroy_pipeline(text_res.pipeline, None);
                            }
                            if text_res.vertex_buffer != vk::Buffer::null() {
                                allocator.destroy_buffer(text_res.vertex_buffer, &mut text_res.vertex_allocation);
                            }
                            // Descriptor set freed by Renderer pool cleanup
                        }
                    } else { info!("TextRenderingResources not found (Core Plugin)."); }

                    // 3. Cleanup GlobalProjectionUboResource (Buffer)
                    if let Some(mut global_ubo_res) = global_ubo_res_opt {
                         unsafe {
                            if global_ubo_res.buffer != vk::Buffer::null() {
                                allocator.destroy_buffer(global_ubo_res.buffer, &mut global_ubo_res.allocation);
                            }
                            // Descriptor set freed by Renderer pool cleanup
                        }
                    } else { info!("GlobalProjectionUboResource not found (Core Plugin)."); }

                    // 4. Cleanup Glyph Atlas (Image, View, Sampler)
                    if let Some(atlas_res) = glyph_atlas_res_opt {
                         match atlas_res.0.lock() {
                            Ok(mut atlas_guard) => {
                                // Pass immutable vk_ctx_guard (or device/allocator directly if needed)
                                atlas_guard.cleanup(&vk_ctx_guard);
                            }
                            Err(poisoned) => error!("GlyphAtlas Mutex poisoned: {:?}", poisoned),
                        }
                    } else { info!("GlyphAtlas resource not found (Core Plugin)."); }

                } else {
                     error!("[Cleanup] Device or Allocator became None after Renderer cleanup!");
                }

                // 5. Cleanup Vulkan Context (Device, Instance, etc. - Must be last)
                // This is called on the vk_ctx_guard which is still valid.
                cleanup_vulkan(&mut vk_ctx_guard);
            }
            Err(poisoned) => {
                error!("VulkanContext Mutex was poisoned before cleanup (Core Plugin): {:?}", poisoned);
            }
        }
    } else {
        warn!("VulkanContext resource not found or already removed during cleanup trigger (Core Plugin).");
    }
}