use bevy_log::{info, error, warn}; 
use ash::vk;
use vk_mem::Alloc;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use bevy_math::Mat4;
use std::collections::HashMap;
use bevy_ecs::prelude::Entity;
use crate::Vertex;
use crate::PreparedDrawData;
use crate::GlobalProjectionUboResource;

struct EntityRenderResources {
    vertex_buffer: vk::Buffer,
    vertex_allocation: vk_mem::Allocation,
    vertex_count: u32,
    offset_uniform: vk::Buffer,
    offset_allocation: vk_mem::Allocation,
    descriptor_set: vk::DescriptorSet,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
struct PipelineCacheKey {
    vertex_shader_path: String,
    fragment_shader_path: String,
}

type ShaderCacheKey = String;


pub struct BufferManager {
    entity_cache: HashMap<Entity, EntityRenderResources>,
    pipeline_cache: HashMap<PipelineCacheKey, vk::Pipeline>,
    shader_cache: HashMap<ShaderCacheKey, vk::ShaderModule>,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
}

impl BufferManager {
    pub fn new(
        platform: &mut VulkanContext,
        descriptor_set_layout: vk::DescriptorSetLayout, // Layout for per-entity sets
        descriptor_pool: vk::DescriptorPool, // Pool for allocating per-entity sets
    ) -> Self {
        // --- Initialize Caches ---
        let entity_cache = HashMap::new();
        let pipeline_cache = HashMap::new();
        let shader_cache = HashMap::new();
        info!("[BufferManager::new] Caches initialized (entity, pipeline, shader)");

        // Note: GlobalProjectionUboResource is created and managed elsewhere (e.g., core plugin startup)
        // BufferManager will expect it to exist when prepare_frame_resources is called.

        Self {
            // Removed: uniform_buffer,
            // Removed: uniform_allocation,
            entity_cache,
            pipeline_cache,
            shader_cache,
            descriptor_set_layout, // Store layout for per-entity sets
            descriptor_pool,       // Store pool for per-entity sets
        }
    }

    // --- prepare_frame_resources with immediate descriptor updates ---
    pub fn prepare_frame_resources(
        &mut self,
        platform: &mut VulkanContext,
        render_commands: &[crate::RenderCommandData], // Includes vertices_changed flag
        global_ubo_res: &GlobalProjectionUboResource,
    ) -> Vec<PreparedDrawData> {
        let device = platform.device.as_ref().expect("Device missing in prepare_frame_resources");
        let allocator = platform.allocator.as_ref().expect("Allocator missing in prepare_frame_resources");
        let render_pass = platform.render_pass.expect("Render pass missing in prepare_frame_resources");
        // BufferManager only deals with shapes for now, so get the shape layout
        let pipeline_layout = platform.shape_pipeline_layout.expect("Shape pipeline layout missing in prepare_frame_resources");

        let mut prepared_draws: Vec<PreparedDrawData> = Vec::with_capacity(render_commands.len());
    
        for command in render_commands {
            let entity_id = command.entity_id;
    
            // Use entry API to handle both existing and new entities cleanly
            let resources = self.entity_cache.entry(entity_id).or_insert_with(|| {
                // --- This closure runs ONLY for New Entities ---
                info!("[BufferManager] Creating resources for Entity {:?}", entity_id);
                let vertex_count = command.vertices.len();
                let vertex_buffer_size = (std::mem::size_of::<Vertex>() * vertex_count) as u64;
    
                // 1. Create Vertex Buffer & Copy Data
                let (vertex_buffer, vertex_allocation) = unsafe {
                    let buffer_info = vk::BufferCreateInfo {
                        s_type: vk::StructureType::BUFFER_CREATE_INFO,
                        size: vertex_buffer_size,
                        usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                        sharing_mode: vk::SharingMode::EXCLUSIVE,
                        ..Default::default()
                    };
                    let allocation_info = vk_mem::AllocationCreateInfo {
                        flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE | vk_mem::AllocationCreateFlags::MAPPED,
                        usage: vk_mem::MemoryUsage::AutoPreferDevice,
                        // Other fields default to 0 / empty / null via Default trait
                        ..Default::default()
                    };
                    allocator.create_buffer(&buffer_info, &allocation_info)
                             .expect("Failed to create vertex buffer")
                };
                unsafe {
                    let info = allocator.get_allocation_info(&vertex_allocation);
                    assert!(!info.mapped_data.is_null(), "Vertex allocation should be mapped but pointer is null");
                    let data_ptr = info.mapped_data.cast::<Vertex>();
                    data_ptr.copy_from_nonoverlapping(command.vertices.as_ptr(), vertex_count);
                }
    
                // 2. Create Offset Uniform Buffer & Copy Data
                let (offset_uniform, offset_allocation) = unsafe {
                     // **** Use ..Default::default() ****
                    let buffer_info = vk::BufferCreateInfo {
                        s_type: vk::StructureType::BUFFER_CREATE_INFO,
                        size: std::mem::size_of::<Mat4>() as u64,
                        usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
                        sharing_mode: vk::SharingMode::EXCLUSIVE,
                        ..Default::default()
                    };
                     // **** Correct AllocationCreateInfo for vk-mem 0.4 ****
                    let allocation_info = vk_mem::AllocationCreateInfo {
                        flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE | vk_mem::AllocationCreateFlags::MAPPED,
                        usage: vk_mem::MemoryUsage::AutoPreferDevice,
                        ..Default::default()
                    };
                    allocator.create_buffer(&buffer_info, &allocation_info)
                             .expect("Failed to create offset uniform buffer")
                };
                unsafe {
                    let info = allocator.get_allocation_info(&offset_allocation);
                    if !info.mapped_data.is_null() {
                        let data_ptr = info.mapped_data.cast::<f32>();
                        data_ptr.copy_from_nonoverlapping(command.transform_matrix.to_cols_array().as_ptr(), 16);
                    } else {
                         error!("[BufferManager] Offset UBO allocation not mapped during initial write for {:?}!", entity_id);
                    }
                }
    
                // 3. Allocate Descriptor Set
                let descriptor_set = unsafe {
                    device.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo {
                        s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
                        descriptor_pool: self.descriptor_pool,
                        descriptor_set_count: 1,
                        p_set_layouts: &self.descriptor_set_layout,
                        ..Default::default() // Handles p_next, _marker
                    }).expect("Failed to allocate descriptor set")[0]
                };
    
                // 4. Load/Get Shaders from Cache
                let vertex_shader_path = &command.vertex_shader_path;
                if !self.shader_cache.contains_key(vertex_shader_path) {
                    info!("[BufferManager] Loading and caching shader: {}", vertex_shader_path);
                    let shader = crate::gui_framework::rendering::shader_utils::load_shader(device, vertex_shader_path);
                    self.shader_cache.insert(vertex_shader_path.clone(), shader);
                }
                let fragment_shader_path = &command.fragment_shader_path;
                 if !self.shader_cache.contains_key(fragment_shader_path) {
                    info!("[BufferManager] Loading and caching shader: {}", fragment_shader_path);
                    let shader = crate::gui_framework::rendering::shader_utils::load_shader(device, fragment_shader_path);
                    self.shader_cache.insert(fragment_shader_path.clone(), shader);
                }
    
                // 5. Create/Get Pipeline from Cache
                let pipeline_key = PipelineCacheKey {
                    vertex_shader_path: command.vertex_shader_path.clone(),
                    fragment_shader_path: command.fragment_shader_path.clone(),
                };
                self.pipeline_cache.entry(pipeline_key.clone()).or_insert_with(|| {
                    info!("[BufferManager] Creating and caching pipeline for key: {:?}", pipeline_key);
                    let vertex_shader = self.shader_cache[&pipeline_key.vertex_shader_path];
                    let fragment_shader = self.shader_cache[&pipeline_key.fragment_shader_path];
                    unsafe {
                        let shader_stages = [
                            vk::PipelineShaderStageCreateInfo { s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO, module: vertex_shader, stage: vk::ShaderStageFlags::VERTEX, p_name: b"main\0".as_ptr() as _, ..Default::default() },
                            vk::PipelineShaderStageCreateInfo { s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO, module: fragment_shader, stage: vk::ShaderStageFlags::FRAGMENT, p_name: b"main\0".as_ptr() as _, ..Default::default() },
                        ];
                        let vertex_attr_descs = [ vk::VertexInputAttributeDescription { location: 0, binding: 0, format: vk::Format::R32G32_SFLOAT, offset: 0 } ];
                        let vertex_binding_descs = [ vk::VertexInputBindingDescription { binding: 0, stride: std::mem::size_of::<Vertex>() as u32, input_rate: vk::VertexInputRate::VERTEX } ];
                        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo { s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO, vertex_binding_description_count: 1, p_vertex_binding_descriptions: vertex_binding_descs.as_ptr(), vertex_attribute_description_count: 1, p_vertex_attribute_descriptions: vertex_attr_descs.as_ptr(), ..Default::default() };
                        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo { s_type: vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO, topology: vk::PrimitiveTopology::TRIANGLE_LIST, ..Default::default() };
                        let viewport_state = vk::PipelineViewportStateCreateInfo { s_type: vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO, viewport_count: 1, scissor_count: 1, ..Default::default() }; // Viewport/Scissor set dynamically
                        let rasterizer = vk::PipelineRasterizationStateCreateInfo { s_type: vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO, polygon_mode: vk::PolygonMode::FILL, line_width: 1.0, cull_mode: vk::CullModeFlags::NONE, front_face: vk::FrontFace::CLOCKWISE, ..Default::default() };
                        let multisampling = vk::PipelineMultisampleStateCreateInfo { s_type: vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO, rasterization_samples: vk::SampleCountFlags::TYPE_1, ..Default::default() };
                        let color_blend_attachment = vk::PipelineColorBlendAttachmentState { // No s_type here
                            blend_enable: vk::FALSE,
                            color_write_mask: vk::ColorComponentFlags::RGBA,
                            ..Default::default() // Use default for blend factors/ops when disabled
                        };
                        let color_blending = vk::PipelineColorBlendStateCreateInfo { s_type: vk::StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO, attachment_count: 1, p_attachments: &color_blend_attachment, ..Default::default() };
                        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
                        let dynamic_state_info = vk::PipelineDynamicStateCreateInfo { s_type: vk::StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO, dynamic_state_count: dynamic_states.len() as u32, p_dynamic_states: dynamic_states.as_ptr(), ..Default::default() };
                        let pipeline_info = vk::GraphicsPipelineCreateInfo {
                            s_type: vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
                            stage_count: shader_stages.len() as u32,
                            p_stages: shader_stages.as_ptr(),
                            p_vertex_input_state: &vertex_input_info,
                            p_input_assembly_state: &input_assembly,
                            p_viewport_state: &viewport_state,
                            p_rasterization_state: &rasterizer,
                            p_multisample_state: &multisampling,
                            p_color_blend_state: &color_blending,
                            p_dynamic_state: &dynamic_state_info,
                            layout: pipeline_layout,
                            render_pass,
                            subpass: 0,
                            ..Default::default() // Handles p_next, flags, base_pipeline etc.
                        };
                        device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None).expect("Failed to create graphics pipeline").remove(0)
                    }
                });
    
                // Return the newly created resources
                EntityRenderResources {
                    vertex_buffer, vertex_allocation,
                    vertex_count: vertex_count as u32,
                    offset_uniform, offset_allocation,
                    descriptor_set,
                }
            });
    
            // --- Update existing entities' buffers IF vertices changed ---
            if command.vertices_changed {
                 info!("[BufferManager] Vertices changed for existing {:?}. Updating vertex buffer.", entity_id);
                 let new_vertex_count = command.vertices.len();
                 let new_size_bytes = (std::mem::size_of::<Vertex>() * new_vertex_count) as u64;
                 let current_alloc_info = allocator.get_allocation_info(&resources.vertex_allocation);
    
                 if new_size_bytes != current_alloc_info.size {
                     warn!("[BufferManager] Vertex buffer size changed for {:?} ({} bytes -> {} bytes). Recreating buffer.",
                           entity_id, current_alloc_info.size, new_size_bytes);
                     unsafe { allocator.destroy_buffer(resources.vertex_buffer, &mut resources.vertex_allocation); }
                     let (new_buffer, new_alloc) = unsafe {
                        let buffer_info = vk::BufferCreateInfo {
                            s_type: vk::StructureType::BUFFER_CREATE_INFO,
                            size: new_size_bytes,
                            usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                            sharing_mode: vk::SharingMode::EXCLUSIVE,
                            ..Default::default()
                        };
                        let allocation_info = vk_mem::AllocationCreateInfo {
                            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE | vk_mem::AllocationCreateFlags::MAPPED,
                            usage: vk_mem::MemoryUsage::AutoPreferDevice,
                            ..Default::default()
                        };
                        allocator.create_buffer(&buffer_info, &allocation_info).expect("Failed to recreate vertex buffer")
                     };
                     resources.vertex_buffer = new_buffer;
                     resources.vertex_allocation = new_alloc;
                 }
                 // Copy data (always needed if vertices_changed is true)
                 unsafe {
                     let info = allocator.get_allocation_info(&resources.vertex_allocation);
                     if !info.mapped_data.is_null() {
                         let data_ptr = info.mapped_data.cast::<Vertex>();
                         data_ptr.copy_from_nonoverlapping(command.vertices.as_ptr(), new_vertex_count);
                     } else { error!("[BufferManager] Vertex buffer allocation not mapped during update for {:?}!", entity_id); }
                 }
                 resources.vertex_count = new_vertex_count as u32;
            }
    
            // --- Update Offset UBO (Always, for both new and existing entities) ---
            unsafe {
                let info = allocator.get_allocation_info(&resources.offset_allocation);
                if !info.mapped_data.is_null() {
                    let data_ptr = info.mapped_data.cast::<f32>();
                    data_ptr.copy_from_nonoverlapping(command.transform_matrix.to_cols_array().as_ptr(), 16);
                } else { error!("[BufferManager] Offset UBO allocation not mapped during update for {:?}!", entity_id); }
            }
    
    
            // --- Update Descriptor Set IMMEDIATELY for this entity ---
            let offset_buffer_info_single = vk::DescriptorBufferInfo {
                buffer: resources.offset_uniform,
                offset: 0,
                range: std::mem::size_of::<Mat4>() as u64,
            };
            let writes_single = [
                // Binding 0: Global Projection (Get buffer info from the resource)
                vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: resources.descriptor_set,
                    dst_binding: 0,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    p_buffer_info: &vk::DescriptorBufferInfo {
                        buffer: global_ubo_res.buffer,
                        offset: 0,
                        range: std::mem::size_of::<Mat4>() as u64,
                    },
                    ..Default::default()
                },
                // Binding 1: Per-Object Offset
                vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: resources.descriptor_set,
                    dst_binding: 1,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    p_buffer_info: &offset_buffer_info_single,
                    ..Default::default()
                },
            ];
            unsafe { device.update_descriptor_sets(&writes_single, &[]); }
        }

        // --- Collect Prepared Draw Data (Retrieve pipeline from cache) ---
        for command in render_commands {
            let pipeline_key = PipelineCacheKey {
                vertex_shader_path: command.vertex_shader_path.clone(),
                fragment_shader_path: command.fragment_shader_path.clone(),
            };
            let pipeline = self.pipeline_cache.get(&pipeline_key)
                .expect("Pipeline should be in cache but wasn't found!");
    
            if let Some(resources) = self.entity_cache.get(&command.entity_id) {
                prepared_draws.push(PreparedDrawData {
                    pipeline: *pipeline,
                    vertex_buffer: resources.vertex_buffer,
                    vertex_count: resources.vertex_count,
                    descriptor_set: resources.descriptor_set,
                });
            } else {
                error!("[BufferManager] Resources not found for Entity {:?} during draw data collection!", command.entity_id);
            }
        }
    
        prepared_draws
    } 

    // --- cleanup() function remains the same ---
    pub fn cleanup(
        &mut self,
        platform: &mut VulkanContext,
    ) {
        info!("[BufferManager::cleanup] Called (&mut self, ECS Rework)");
        let device = platform.device.as_ref().expect("Device missing in cleanup");
        let allocator = platform.allocator.as_ref().expect("Allocator missing in cleanup");

        unsafe {
            // Cleanup cached resources
            info!("[BufferManager::cleanup] Cleaning up {} cached entity resources...", self.entity_cache.len());
            let sets_to_free: Vec<vk::DescriptorSet> = self.entity_cache.values()
                .map(|r| r.descriptor_set)
                .collect();

            if !sets_to_free.is_empty() {
                 match device.free_descriptor_sets(self.descriptor_pool, &sets_to_free) {
                    Ok(_) => info!("[BufferManager::cleanup] Freed {} cached descriptor sets", sets_to_free.len()),
                    Err(e) => error!("[BufferManager::cleanup] Failed to free descriptor sets: {:?}", e),
                 }
            } else {
                 info!("[BufferManager::cleanup] No cached descriptor sets to free.");
            }

           // Cleanup entity-specific resources
           let entity_count = self.entity_cache.len();
           for (entity_id, mut resources) in self.entity_cache.drain() {
               info!(
                   "[BufferManager::cleanup] Destroying Entity {:?} Vertex Buffer: {:?}, Allocation: {:?}",
                   entity_id, resources.vertex_buffer, resources.vertex_allocation
               );
               allocator.destroy_buffer(resources.vertex_buffer, &mut resources.vertex_allocation);

               info!(
                   "[BufferManager::cleanup] Destroying Entity {:?} Offset Uniform: {:?}, Allocation: {:?}",
                   entity_id, resources.offset_uniform, resources.offset_allocation
               );
               allocator.destroy_buffer(resources.offset_uniform, &mut resources.offset_allocation);
           }
           info!("[BufferManager::cleanup] Cleaned up {} entity-specific resources.", entity_count);

           // Cleanup cached pipelines
           let pipeline_count = self.pipeline_cache.len();
           info!("[BufferManager::cleanup] Cleaning up {} cached pipelines...", pipeline_count);
           for (key, pipeline) in self.pipeline_cache.drain() {
                info!("[BufferManager::cleanup] Destroying Pipeline: {:?} ({:?})", key, pipeline);
                device.destroy_pipeline(pipeline, None);
           }

           // Cleanup cached shaders
           let shader_count = self.shader_cache.len();
           info!("[BufferManager::cleanup] Cleaning up {} cached shaders...", shader_count);
           for (key, shader_module) in self.shader_cache.drain() {
                info!("[BufferManager::cleanup] Destroying Shader: {} ({:?})", key, shader_module);
                device.destroy_shader_module(shader_module, None);
           }

            // Global uniform buffer cleanup handled where GlobalProjectionUboResource is managed

        }
        info!("[BufferManager::cleanup] Finished");
    }
}