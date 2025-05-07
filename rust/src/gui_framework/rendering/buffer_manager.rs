use bevy_log::{info, error, warn};
use ash::vk;
use vk_mem::Alloc;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use bevy_math::Mat4;
use std::collections::HashMap;
use bevy_ecs::prelude::Entity;
use crate::{Vertex, Color}; // Import Vertex and Color
use crate::{PreparedDrawData, RenderCommandData}; // Import command/prepared data structs
use crate::GlobalProjectionUboResource;
use crate::gui_framework::rendering::shader_utils; // Keep shader_utils for loading the single shader set
use bevy_color::ColorToComponents;
use std::sync::Arc;

// Struct holding Vulkan resources specific to one entity
struct EntityRenderResources {
    vertex_buffer: vk::Buffer,
    vertex_allocation: vk_mem::Allocation,
    vertex_count: u32,
    offset_uniform: vk::Buffer, // Renamed from transform_ubo for clarity
    offset_allocation: vk_mem::Allocation, // Renamed from transform_alloc
    descriptor_set: vk::DescriptorSet, // Set 0 (Global UBO, Offset UBO)
}

// Key for caching pipelines. Currently only one shape pipeline exists.
// Kept for potential future variations (e.g., blend modes, wireframe).
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
struct PipelineCacheKey {
    // Placeholder for future state variations
    id: u32, // Simple ID for now, always 0
}

// REMOVED: type ShaderCacheKey = String;

pub struct BufferManager {
    // Renamed entity_cache to entity_resources for clarity
    entity_resources: HashMap<Entity, EntityRenderResources>,
    pipeline_cache: HashMap<PipelineCacheKey, vk::Pipeline>,
    // REMOVED: shader_cache: HashMap<ShaderCacheKey, vk::ShaderModule>,
    per_entity_layout: vk::DescriptorSetLayout, // Layout for Set 0
    descriptor_pool: vk::DescriptorPool,
}

impl BufferManager {
    pub fn new(
        _platform: &mut VulkanContext, // Mark platform as unused for now if only needed for handles
        per_entity_layout: vk::DescriptorSetLayout,
        descriptor_pool: vk::DescriptorPool, // Pool for allocating per-entity sets
    ) -> Self {
        // --- Initialize Caches ---
        let entity_resources = HashMap::new();
        let pipeline_cache = HashMap::new();
        // REMOVED: let shader_cache = HashMap::new();

        Self {
            entity_resources,
            pipeline_cache,
            // REMOVED: shader_cache,
            per_entity_layout, // Store layout for per-entity sets
            descriptor_pool,       // Store pool for per-entity sets
        }
    }

    // --- prepare_frame_resources with push constants ---
    pub fn prepare_frame_resources(
        &mut self,
        platform: &mut VulkanContext,
        render_commands: &[RenderCommandData], // Use updated RenderCommandData
        global_ubo_res: &GlobalProjectionUboResource,
    ) -> Vec<PreparedDrawData> {
        let device = platform.device.as_ref().expect("Device missing in prepare_frame_resources");
        let allocator = platform.allocator.as_ref().expect("Allocator missing in prepare_frame_resources");
        let render_pass = platform.render_pass.expect("Render pass missing in prepare_frame_resources");
        // Get the shape pipeline layout (includes push constant range)
        let pipeline_layout = platform.shape_pipeline_layout.expect("Shape pipeline layout missing in prepare_frame_resources");

        let mut prepared_draws: Vec<PreparedDrawData> = Vec::with_capacity(render_commands.len());

        for command in render_commands {
            let entity_id = command.entity_id;
            let mut entity_existed = true; // Flag to track if entity was new this frame

            // --- Get or Create Entity Resources ---
            if !self.entity_resources.contains_key(&entity_id) {
                entity_existed = false;
                let vertex_count = command.vertices.len();
                let vertex_buffer_size = (std::mem::size_of::<Vertex>() * vertex_count) as u64;

                // 1. Create Vertex Buffer & Copy Data
                let (vertex_buffer, vertex_allocation) = unsafe {
                    let buffer_info = vk::BufferCreateInfo { s_type: vk::StructureType::BUFFER_CREATE_INFO, size: vertex_buffer_size, usage: vk::BufferUsageFlags::VERTEX_BUFFER, sharing_mode: vk::SharingMode::EXCLUSIVE, ..Default::default() };
                    let allocation_info = vk_mem::AllocationCreateInfo { flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE | vk_mem::AllocationCreateFlags::MAPPED, usage: vk_mem::MemoryUsage::AutoPreferDevice, ..Default::default() };
                    allocator.create_buffer(&buffer_info, &allocation_info).expect("Failed to create vertex buffer")
                };
                unsafe {
                    let info = allocator.get_allocation_info(&vertex_allocation); assert!(!info.mapped_data.is_null(), "Vertex allocation should be mapped");
                    info.mapped_data.cast::<Vertex>().copy_from_nonoverlapping(command.vertices.as_ptr(), vertex_count);
                }

                // 2. Create Offset Uniform Buffer & Copy Data
                let (offset_uniform, offset_allocation) = unsafe {
                    let buffer_info = vk::BufferCreateInfo { s_type: vk::StructureType::BUFFER_CREATE_INFO, size: std::mem::size_of::<Mat4>() as u64, usage: vk::BufferUsageFlags::UNIFORM_BUFFER, sharing_mode: vk::SharingMode::EXCLUSIVE, ..Default::default() };
                    let allocation_info = vk_mem::AllocationCreateInfo { flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE | vk_mem::AllocationCreateFlags::MAPPED, usage: vk_mem::MemoryUsage::AutoPreferDevice, ..Default::default() };
                    allocator.create_buffer(&buffer_info, &allocation_info).expect("Failed to create offset uniform buffer")
                };
                unsafe {
                    let info = allocator.get_allocation_info(&offset_allocation);
                    if !info.mapped_data.is_null() { info.mapped_data.cast::<f32>().copy_from_nonoverlapping(command.transform_matrix.to_cols_array().as_ptr(), 16); }
                    else { error!("[BufferManager] Offset UBO allocation not mapped during initial write for {:?}!", entity_id); }
                }

                // 3. Allocate Descriptor Set (Set 0)
                let descriptor_set = unsafe {
                    device.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo { s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO, descriptor_pool: self.descriptor_pool, descriptor_set_count: 1, p_set_layouts: &self.per_entity_layout, ..Default::default() })
                          .expect("Failed to allocate descriptor set")[0]
                };

                // REMOVED: Shader loading/caching logic

                // Insert the newly created resources into the cache
                self.entity_resources.insert(entity_id, EntityRenderResources {
                    vertex_buffer, vertex_allocation,
                    vertex_count: vertex_count as u32,
                    offset_uniform, offset_allocation,
                    descriptor_set,
                });
            }

            // --- Update Existing Entity Resources ---
            if entity_existed {
                let resources = self.entity_resources.get_mut(&entity_id).unwrap();

                // --- Update Vertex Buffer IF vertices changed ---
                if command.vertices_changed {
                    let new_vertex_count = command.vertices.len();
                    let new_size_bytes = (std::mem::size_of::<Vertex>() * new_vertex_count) as u64;
                    let current_alloc_info = allocator.get_allocation_info(&resources.vertex_allocation);

                    if new_size_bytes != current_alloc_info.size {
                        warn!("[BufferManager] Vertex buffer size changed for {:?} ({} bytes -> {} bytes). Recreating buffer.",
                              entity_id, current_alloc_info.size, new_size_bytes);
                        unsafe { allocator.destroy_buffer(resources.vertex_buffer, &mut resources.vertex_allocation); }
                        let (new_buffer, new_alloc) = unsafe {
                            let buffer_info = vk::BufferCreateInfo { s_type: vk::StructureType::BUFFER_CREATE_INFO, size: new_size_bytes, usage: vk::BufferUsageFlags::VERTEX_BUFFER, sharing_mode: vk::SharingMode::EXCLUSIVE, ..Default::default() };
                            let allocation_info = vk_mem::AllocationCreateInfo { flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE | vk_mem::AllocationCreateFlags::MAPPED, usage: vk_mem::MemoryUsage::AutoPreferDevice, ..Default::default() };
                            allocator.create_buffer(&buffer_info, &allocation_info).expect("Failed to recreate vertex buffer")
                        };
                        resources.vertex_buffer = new_buffer;
                        resources.vertex_allocation = new_alloc;
                    }
                    // Copy data (always needed if vertices_changed is true)
                    unsafe {
                        let info = allocator.get_allocation_info(&resources.vertex_allocation);
                        if !info.mapped_data.is_null() {
                            info.mapped_data.cast::<Vertex>().copy_from_nonoverlapping(command.vertices.as_ptr(), new_vertex_count);
                        } else { error!("[BufferManager] Vertex buffer allocation not mapped during update for {:?}!", entity_id); }
                    }
                    resources.vertex_count = new_vertex_count as u32; // Update vertex count only if vertices changed
                }

                // --- Update Offset Uniform Buffer (Always) ---
                unsafe {
                    let info = allocator.get_allocation_info(&resources.offset_allocation);
                    if !info.mapped_data.is_null() {
                        info.mapped_data.cast::<f32>().copy_from_nonoverlapping(command.transform_matrix.to_cols_array().as_ptr(), 16);
                    } else {
                        error!("[BufferManager] Offset UBO allocation not mapped during update for {:?}!", entity_id);
                    }
                    // --- Add Explicit Flush ---
                    if let Err(e) = allocator.flush_allocation(&resources.offset_allocation, 0, vk::WHOLE_SIZE) {
                        error!("[BufferManager] Failed to flush offset UBO allocation for {:?}: {:?}", entity_id, e);
                    }
                }
            }

            // --- Update Descriptor Set (Always, for both new and existing entities) ---
            // This needs to happen *after* potential buffer recreation/update
            // Get the resources again (might have been created above if new)
            let resources = self.entity_resources.get(&entity_id).unwrap(); // Should always exist now
            let offset_buffer_info_single = vk::DescriptorBufferInfo { buffer: resources.offset_uniform, offset: 0, range: std::mem::size_of::<Mat4>() as u64 };
            let writes_single = [
                // Binding 0: Global UBO
                vk::WriteDescriptorSet { s_type: vk::StructureType::WRITE_DESCRIPTOR_SET, dst_set: resources.descriptor_set, dst_binding: 0, descriptor_count: 1, descriptor_type: vk::DescriptorType::UNIFORM_BUFFER, p_buffer_info: &vk::DescriptorBufferInfo { buffer: global_ubo_res.buffer, offset: 0, range: std::mem::size_of::<Mat4>() as u64 }, ..Default::default() },
                // Binding 1: Offset UBO
                vk::WriteDescriptorSet { s_type: vk::StructureType::WRITE_DESCRIPTOR_SET, dst_set: resources.descriptor_set, dst_binding: 1, descriptor_count: 1, descriptor_type: vk::DescriptorType::UNIFORM_BUFFER, p_buffer_info: &offset_buffer_info_single, ..Default::default() },
            ];
            unsafe { device.update_descriptor_sets(&writes_single, &[]); }

            // --- Get or Create the single Shape Pipeline ---
            let pipeline_key = PipelineCacheKey { id: 0 }; // Use constant key
            let pipeline = *self.pipeline_cache.entry(pipeline_key).or_insert_with(|| {
                // Load shaders
                let vert_shader_module = shader_utils::load_shader(device, "shape.vert.spv");
                let frag_shader_module = shader_utils::load_shader(device, "shape.frag.spv");

                // Create pipeline (using logic from previous attempt)
                let pipeline = unsafe {
                    let shader_stages = [ vk::PipelineShaderStageCreateInfo { s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO, module: vert_shader_module, stage: vk::ShaderStageFlags::VERTEX, p_name: b"main\0".as_ptr() as _, ..Default::default() }, vk::PipelineShaderStageCreateInfo { s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO, module: frag_shader_module, stage: vk::ShaderStageFlags::FRAGMENT, p_name: b"main\0".as_ptr() as _, ..Default::default() }, ];
                    let vertex_attr_descs = [ vk::VertexInputAttributeDescription { location: 0, binding: 0, format: vk::Format::R32G32_SFLOAT, offset: 0 }, ];
                    let vertex_binding_descs = [ vk::VertexInputBindingDescription { binding: 0, stride: std::mem::size_of::<Vertex>() as u32, input_rate: vk::VertexInputRate::VERTEX } ];
                    let vertex_input_info = vk::PipelineVertexInputStateCreateInfo { s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO, vertex_binding_description_count: vertex_binding_descs.len() as u32, p_vertex_binding_descriptions: vertex_binding_descs.as_ptr(), vertex_attribute_description_count: vertex_attr_descs.len() as u32, p_vertex_attribute_descriptions: vertex_attr_descs.as_ptr(), ..Default::default() };
                    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo { s_type: vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO, topology: vk::PrimitiveTopology::TRIANGLE_LIST, ..Default::default() };
                    let viewport_state = vk::PipelineViewportStateCreateInfo { s_type: vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO, viewport_count: 1, scissor_count: 1, ..Default::default() };
                    let rasterizer = vk::PipelineRasterizationStateCreateInfo { s_type: vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO, polygon_mode: vk::PolygonMode::FILL, line_width: 1.0, cull_mode: vk::CullModeFlags::NONE, front_face: vk::FrontFace::CLOCKWISE, ..Default::default() };
                    let multisampling = vk::PipelineMultisampleStateCreateInfo { s_type: vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO, rasterization_samples: vk::SampleCountFlags::TYPE_1, ..Default::default() };
                    let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo { s_type: vk::StructureType::PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO, depth_test_enable: vk::TRUE, depth_write_enable: vk::TRUE, depth_compare_op: vk::CompareOp::LESS, depth_bounds_test_enable: vk::FALSE, stencil_test_enable: vk::FALSE, ..Default::default() };
                    let color_blend_attachment = vk::PipelineColorBlendAttachmentState { blend_enable: vk::TRUE, src_color_blend_factor: vk::BlendFactor::SRC_ALPHA, dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA, color_blend_op: vk::BlendOp::ADD, src_alpha_blend_factor: vk::BlendFactor::ONE, dst_alpha_blend_factor: vk::BlendFactor::ZERO, alpha_blend_op: vk::BlendOp::ADD, color_write_mask: vk::ColorComponentFlags::RGBA, };
                    let color_blending = vk::PipelineColorBlendStateCreateInfo { s_type: vk::StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO, logic_op_enable: vk::FALSE, attachment_count: 1, p_attachments: &color_blend_attachment, ..Default::default() };
                    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
                    let dynamic_state_info = vk::PipelineDynamicStateCreateInfo { s_type: vk::StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO, dynamic_state_count: dynamic_states.len() as u32, p_dynamic_states: dynamic_states.as_ptr(), ..Default::default() };
                    let pipeline_info = vk::GraphicsPipelineCreateInfo { s_type: vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO, stage_count: shader_stages.len() as u32, p_stages: shader_stages.as_ptr(), p_vertex_input_state: &vertex_input_info, p_input_assembly_state: &input_assembly, p_viewport_state: &viewport_state, p_rasterization_state: &rasterizer, p_multisample_state: &multisampling, p_color_blend_state: &color_blending, p_depth_stencil_state: &depth_stencil_state, p_dynamic_state: &dynamic_state_info, layout: pipeline_layout, render_pass, subpass: 0, ..Default::default() };
                    device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None).expect("Failed to create shape graphics pipeline").remove(0)
                };
                // Cleanup shader modules immediately
                unsafe {
                    device.destroy_shader_module(vert_shader_module, None);
                    device.destroy_shader_module(frag_shader_module, None);
                }
                pipeline
            });


            // --- Collect Prepared Draw Data ---
            let resources = self.entity_resources.get(&entity_id).unwrap(); // Get resources again

            // Convert Bevy Color to [f32; 4] for push constants
            let color_rgba = match command.color {
                 Color::Srgba(c) => c.to_f32_array(),
                 Color::LinearRgba(c) => c.to_f32_array(),
                 _ => Color::WHITE.to_srgba().to_f32_array(), // Fallback
            };

            prepared_draws.push(PreparedDrawData {
                pipeline, // Use the pipeline retrieved/created above
                vertex_buffer: resources.vertex_buffer,
                vertex_count: resources.vertex_count,
                descriptor_set: resources.descriptor_set,
                color: color_rgba, // Add color data
            });
        } // End of loop through render_commands

        prepared_draws
    }

    /// Removes Vulkan resources associated with a specific entity.
    /// Called when an entity with ShapeData or CursorVisual is despawned.
    pub fn remove_entity_resources(
        &mut self,
        entity: Entity,
        device: &ash::Device,
        allocator: &Arc<vk_mem::Allocator>,
    ) {
        if let Some(mut resources) = self.entity_resources.remove(&entity) {
            info!("[BufferManager] Removing resources for despawned entity {:?}", entity);
            unsafe {
                allocator.destroy_buffer(resources.vertex_buffer, &mut resources.vertex_allocation);
                allocator.destroy_buffer(resources.offset_uniform, &mut resources.offset_allocation);
                if resources.descriptor_set != vk::DescriptorSet::null() {
                    // The descriptor_pool was created with FREE_DESCRIPTOR_SET flag
                    if let Err(e) = device.free_descriptor_sets(self.descriptor_pool, &[resources.descriptor_set]) {
                        error!("[BufferManager] Failed to free descriptor set for entity {:?}: {:?}", entity, e);
                } else {
                    info!("[BufferManager] Freed descriptor set for entity {:?}", entity);
                }
            }
        }
    } else {
        // This might happen if cleanup is called multiple times for the same entity,
        // or if the entity never had shape resources managed by BufferManager.
        // It's not necessarily an error, so a warn! might be too noisy.
        // A debug! log could be appropriate if needed for diagnostics.
        // info!("[BufferManager] Attempted to remove resources for entity {:?}, but it was not found in cache.", entity);
        }
    }

    // --- cleanup() function ---
    // This is for full resource cleanup on app exit
    pub fn cleanup(
        &mut self,
        platform: &mut VulkanContext, // Keep VulkanContext here for now, or change to device/allocator
    ) {
        let device = platform.device.as_ref().expect("Device missing in BufferManager::cleanup");
        let allocator = platform.allocator.as_ref().expect("Allocator missing in BufferManager::cleanup");

        // This part is tricky: if descriptor_pool is shared, we should not destroy it here.
        // PipelineManager creates it and gives it to Renderer, which then gives it to BufferManager.
        // Renderer should own and destroy the pool.
        // BufferManager should only free sets it allocated from that pool.

        let mut sets_to_free: Vec<vk::DescriptorSet> = Vec::new();
        unsafe {
            // Cleanup entity-specific resources
            let entity_count = self.entity_resources.len();
            for (_entity_id, mut resources) in self.entity_resources.drain() {
                allocator.destroy_buffer(resources.vertex_buffer, &mut resources.vertex_allocation);
                allocator.destroy_buffer(resources.offset_uniform, &mut resources.offset_allocation);
                if resources.descriptor_set != vk::DescriptorSet::null() {
                    sets_to_free.push(resources.descriptor_set);
                }
            }
            if !sets_to_free.is_empty() {
                if let Err(e) = device.free_descriptor_sets(self.descriptor_pool, &sets_to_free) {
                    error!("[BufferManager::cleanup] Failed to free {} descriptor sets during full cleanup: {:?}", sets_to_free.len(), e);
                } else {
                    info!("[BufferManager::cleanup] Freed {} descriptor sets during full cleanup.", sets_to_free.len());
                }
            }
            info!("[BufferManager::cleanup] Cleaned up resources for {} entities.", entity_count);

            // Cleanup cached pipelines
            let pipeline_count = self.pipeline_cache.len();
            for (_key, pipeline) in self.pipeline_cache.drain() {
                device.destroy_pipeline(pipeline, None);
            }
            info!("[BufferManager::cleanup] Cleaned up {} cached pipelines.", pipeline_count);
        }
    }
}