use bevy_log::{warn, info, error};
use ash::vk;
use vk_mem::Alloc; // Import Allocation directly
use crate::gui_framework::context::vulkan_context::VulkanContext;
// Removed Renderable import
// use crate::gui_framework::rendering::renderable::Renderable;
use bevy_math::Mat4;
use std::collections::HashMap; // Needed for caching later
use bevy_ecs::prelude::Entity; // Needed for caching later
use crate::Vertex; // Needed for vertex buffer size

// Holds the Vulkan resources cached per entity.
// TODO: Optimize pipeline/shader caching later.
struct EntityRenderResources {
    vertex_buffer: vk::Buffer,
    vertex_allocation: vk_mem::Allocation, // Store Allocation directly
    vertex_count: u32,
    offset_uniform: vk::Buffer,
    offset_allocation: vk_mem::Allocation,
    descriptor_set: vk::DescriptorSet,     // Per-entity descriptor set
    pipeline: vk::Pipeline,                // Per-entity pipeline (inefficient, cache later)
    vertex_shader: vk::ShaderModule,
    fragment_shader: vk::ShaderModule,
    // Add instancing fields later
}

pub struct BufferManager {
    pub uniform_buffer: vk::Buffer,
    pub uniform_allocation: vk_mem::Allocation, // Global uniform is always valid
    // Replace renderables Vec with a cache
    entity_cache: HashMap<Entity, EntityRenderResources>,
    // Store layout/pool needed for creating new descriptor sets
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
}

impl BufferManager {
    // Modified signature: Takes &mut VulkanContext
    pub fn new(
        platform: &mut VulkanContext,
        _pipeline_layout: vk::PipelineLayout, // Mark unused for now
        descriptor_set_layout: vk::DescriptorSetLayout, // Store this
        descriptor_pool: vk::DescriptorPool, // Store this
    ) -> Self {
        info!("[BufferManager::new] Called (ECS Migration - Reworking)");
        let allocator = platform.allocator.as_ref().unwrap();

        // --- Uniform Buffer Setup (Keep this part) ---
        let ortho = Mat4::orthographic_rh(0.0, 600.0, 300.0, 0.0, -1.0, 1.0); // Use default size initially
        let (uniform_buffer, uniform_allocation) = {
            let buffer_info = vk::BufferCreateInfo {
                s_type: vk::StructureType::BUFFER_CREATE_INFO,
                size: std::mem::size_of::<Mat4>() as u64,
                usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            };
            let allocation_info = vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                flags: vk_mem::AllocationCreateFlags::MAPPED | vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
                ..Default::default()
            };
            unsafe {
                match allocator.create_buffer(&buffer_info, &allocation_info) {
                    Ok((buffer, mut allocation)) => {
                        let data_ptr = allocator.map_memory(&mut allocation).unwrap().cast::<f32>();
                        data_ptr.copy_from_nonoverlapping(ortho.to_cols_array().as_ptr(), 16);
                        allocator.unmap_memory(&mut allocation);
                        (buffer, allocation)
                    }
                    Err(e) => panic!("Uniform buffer creation failed: {:?}", e),
                }
            }
        };
        info!("[BufferManager::new] Initial uniform buffer created");

        // --- Initialize Cache ---
        let entity_cache = HashMap::new();
        warn!("[BufferManager::new] Entity resource cache initialized (empty)");

        Self {
            uniform_buffer,
            uniform_allocation,
            entity_cache,
            descriptor_set_layout, // Store for later use
            descriptor_pool,     // Store for later use
        }
    }

    /// Prepares Vulkan resources for the current frame based on ECS data.
    /// Creates resources for new entities and updates existing ones.
    /// Returns a list of `PreparedDrawData` for the rendering system.
    pub fn prepare_frame_resources(
        &mut self,
        platform: &mut VulkanContext,
        render_commands: &[crate::RenderCommandData],
    ) -> Vec<crate::PreparedDrawData> {
        let device = platform.device.as_ref().expect("Device missing in prepare_frame_resources");
        let allocator = platform.allocator.as_ref().expect("Allocator missing in prepare_frame_resources");
        let render_pass = platform.render_pass.expect("Render pass missing in prepare_frame_resources");
        let pipeline_layout = platform.pipeline_layout.expect("Pipeline layout missing in prepare_frame_resources"); // Assuming layout is stored in VulkanContext now

        let mut prepared_draws = Vec::with_capacity(render_commands.len());
        let mut created_entities = Vec::new(); // Track entities created this frame for descriptor updates

        for command in render_commands {
            let entity_id = command.entity_id;

            // --- Create or Get Cached Resources ---
            if !self.entity_cache.contains_key(&entity_id) {
                info!("[BufferManager] Creating resources for Entity {:?}", entity_id);

                // 1. Create Vertex Buffer
                let vertex_buffer_size = (std::mem::size_of::<Vertex>() * command.vertices.len()) as u64;
                let (vertex_buffer, mut vertex_allocation) = unsafe {
                    let buffer_info = vk::BufferCreateInfo {
                        s_type: vk::StructureType::BUFFER_CREATE_INFO,
                        size: vertex_buffer_size,
                        usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                        sharing_mode: vk::SharingMode::EXCLUSIVE,
                        ..Default::default()
                    };
                    let allocation_info = vk_mem::AllocationCreateInfo {
                        usage: vk_mem::MemoryUsage::AutoPreferDevice, // Let vk-mem decide best place
                        flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE | vk_mem::AllocationCreateFlags::MAPPED, // Keep mapped for potential updates
                        ..Default::default()
                    };
                    allocator.create_buffer(&buffer_info, &allocation_info)
                        .expect("Failed to create vertex buffer")
                };
                // Copy initial vertex data using the persistently mapped pointer via AllocationInfo
                unsafe {
                    // Get AllocationInfo from the allocator to access the mapped pointer
                    let info = allocator.get_allocation_info(&vertex_allocation);
                    assert!(!info.mapped_data.is_null(), "Vertex allocation should be mapped but pointer is null");
                    let data_ptr = info.mapped_data.cast::<Vertex>(); // Directly cast the raw
                    data_ptr.copy_from_nonoverlapping(command.vertices.as_ptr(), command.vertices.len());
                    // No need to unmap persistently mapped memory
                }


                // 2. Create Offset Uniform Buffer
                let (offset_uniform, offset_allocation) = unsafe {
                    let buffer_info = vk::BufferCreateInfo {
                        s_type: vk::StructureType::BUFFER_CREATE_INFO,
                        size: std::mem::size_of::<Mat4>() as u64,
                        usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
                        sharing_mode: vk::SharingMode::EXCLUSIVE,
                        ..Default::default()
                    };
                    let allocation_info = vk_mem::AllocationCreateInfo {
                        usage: vk_mem::MemoryUsage::AutoPreferDevice,
                        flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE | vk_mem::AllocationCreateFlags::MAPPED,
                        ..Default::default()
                    };
                    allocator.create_buffer(&buffer_info, &allocation_info)
                        .expect("Failed to create offset uniform buffer")
                };

                // 3. Allocate Descriptor Set
                let descriptor_set = unsafe {
                    device.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo {
                        s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
                        descriptor_pool: self.descriptor_pool,
                        descriptor_set_count: 1,
                        p_set_layouts: &self.descriptor_set_layout,
                        ..Default::default()
                    }).expect("Failed to allocate descriptor set")[0]
                };

                // 4. Load Shaders
                // TODO: Cache shaders based on path
                let vertex_shader = crate::gui_framework::rendering::shader_utils::load_shader(device, &command.vertex_shader_path);
                let fragment_shader = crate::gui_framework::rendering::shader_utils::load_shader(device, &command.fragment_shader_path);

                // 5. Create Pipeline
                // TODO: Cache pipelines based on shaders/renderpass/etc.
                warn!("[BufferManager] Creating unique pipeline for Entity {:?}. Consider caching.", entity_id);
                let pipeline = unsafe {
                    let shader_stages = [
                        vk::PipelineShaderStageCreateInfo {
                            s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                            stage: vk::ShaderStageFlags::VERTEX,
                            module: vertex_shader,
                            p_name: b"main\0".as_ptr() as *const std::ffi::c_char,
                            ..Default::default()
                        },
                        vk::PipelineShaderStageCreateInfo {
                            s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                            stage: vk::ShaderStageFlags::FRAGMENT,
                            module: fragment_shader,
                            p_name: b"main\0".as_ptr() as *const std::ffi::c_char,
                            ..Default::default()
                        },
                    ];
                    let vertex_binding_desc = vk::VertexInputBindingDescription {
                        binding: 0, // Vertex data
                        stride: std::mem::size_of::<Vertex>() as u32,
                        input_rate: vk::VertexInputRate::VERTEX,
                    };
                    let vertex_attr_desc = vk::VertexInputAttributeDescription {
                        location: 0, // layout(location = 0) in vec2 pos;
                        binding: 0,
                        format: vk::Format::R32G32_SFLOAT,
                        offset: 0,
                    };
                    // Add instancing binding/attributes later if needed (binding 1)

                    let vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
                        s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
                        vertex_binding_description_count: 1,
                        p_vertex_binding_descriptions: &vertex_binding_desc,
                        vertex_attribute_description_count: 1,
                        p_vertex_attribute_descriptions: &vertex_attr_desc,
                        ..Default::default()
                    };
                    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
                        s_type: vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
                        topology: vk::PrimitiveTopology::TRIANGLE_LIST, // Assuming triangles for now
                        primitive_restart_enable: vk::FALSE,
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
                        depth_clamp_enable: vk::FALSE,
                        rasterizer_discard_enable: vk::FALSE,
                        polygon_mode: vk::PolygonMode::FILL,
                        line_width: 1.0,
                        cull_mode: vk::CullModeFlags::NONE, // No culling for 2D
                        front_face: vk::FrontFace::CLOCKWISE,
                        depth_bias_enable: vk::FALSE,
                        ..Default::default()
                    };
                    let multisampling = vk::PipelineMultisampleStateCreateInfo {
                        s_type: vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
                        sample_shading_enable: vk::FALSE,
                        rasterization_samples: vk::SampleCountFlags::TYPE_1,
                        ..Default::default()
                    };
                    let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
                        color_write_mask: vk::ColorComponentFlags::RGBA,
                        blend_enable: vk::FALSE, // Basic opaque blending for now
                        // Add alpha blending settings here if needed later
                        ..Default::default()
                    };
                    let color_blending = vk::PipelineColorBlendStateCreateInfo {
                        s_type: vk::StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
                        logic_op_enable: vk::FALSE,
                        attachment_count: 1,
                        p_attachments: &color_blend_attachment,
                        ..Default::default()
                    };
                     let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
                     let dynamic_state_info = vk::PipelineDynamicStateCreateInfo {
                         s_type: vk::StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO,
                         dynamic_state_count: dynamic_states.len() as u32,
                         p_dynamic_states: dynamic_states.as_ptr(),
                         ..Default::default()
                     };

                    let pipeline_info = vk::GraphicsPipelineCreateInfo {
                        s_type: vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
                        stage_count: shader_stages.len() as u32,
                        p_stages: shader_stages.as_ptr(),
                        p_vertex_input_state: &vertex_input_info,
                        p_input_assembly_state: &input_assembly,
                        p_viewport_state: &viewport_state,
                        p_rasterization_state: &rasterizer,
                        p_multisample_state: &multisampling,
                        p_depth_stencil_state: std::ptr::null(), // No depth/stencil for now
                        p_color_blend_state: &color_blending,
                        p_dynamic_state: &dynamic_state_info,
                        layout: pipeline_layout,
                        render_pass,
                        subpass: 0,
                        ..Default::default()
                    };
                    device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                        .expect("Failed to create graphics pipeline")
                        .remove(0) // We create only one
                };

                // 6. Store in cache
                let resources = EntityRenderResources {
                    vertex_buffer, vertex_allocation,
                    vertex_count: command.vertices.len() as u32,
                    offset_uniform, offset_allocation,
                    descriptor_set, pipeline,
                    vertex_shader, fragment_shader,
                };
                self.entity_cache.insert(entity_id, resources);
                created_entities.push(entity_id); // Mark for descriptor update
            }

             // --- Update Offset UBO (Always) ---
            if let Some(resources) = self.entity_cache.get_mut(&entity_id) {
                // Use the persistently mapped pointer via AllocationInfo
                unsafe {
                    // Get AllocationInfo from the allocator to access the mapped pointer
                    let info = allocator.get_allocation_info(&resources.offset_allocation);
                    assert!(!info.mapped_data.is_null(), "Offset allocation should be mapped but pointer is null");
                    let data_ptr = info.mapped_data.cast::<f32>(); // Directly cast the raw pointer
                    data_ptr.copy_from_nonoverlapping(command.transform_matrix.to_cols_array().as_ptr(), 16);
                    // No need to unmap persistently mapped memory
                }
            }
        } // End of render_commands loop

        // --- Update Descriptor Sets for New/Existing Entities ---
        let mut writes = Vec::with_capacity(render_commands.len() * 2); // Max 2 writes per entity
        let mut proj_buffer_info: Option<vk::DescriptorBufferInfo> = None; // Cache global info

        for command in render_commands {
             if let Some(resources) = self.entity_cache.get(&command.entity_id) {
                 // Cache global projection buffer info if not already done
                 if proj_buffer_info.is_none() {
                     proj_buffer_info = Some(vk::DescriptorBufferInfo {
                         buffer: self.uniform_buffer,
                         offset: 0,
                         range: std::mem::size_of::<Mat4>() as u64,
                     });
                 }

                 // Per-entity offset buffer info
                 let offset_buffer_info = vk::DescriptorBufferInfo {
                     buffer: resources.offset_uniform,
                     offset: 0,
                     range: std::mem::size_of::<Mat4>() as u64,
                 };

                 // Write for Binding 0 (Global Projection)
                 writes.push(vk::WriteDescriptorSet {
                     s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                     dst_set: resources.descriptor_set,
                     dst_binding: 0,
                     descriptor_count: 1,
                     descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                     p_buffer_info: proj_buffer_info.as_ref().unwrap(), // Use cached info
                     ..Default::default()
                 });

                 // Write for Binding 1 (Per-Object Offset)
                 writes.push(vk::WriteDescriptorSet {
                     s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                     dst_set: resources.descriptor_set,
                     dst_binding: 1,
                     descriptor_count: 1,
                     descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                     p_buffer_info: &offset_buffer_info,
                     ..Default::default()
                 });
             }
        }
        if !writes.is_empty() {
             unsafe { device.update_descriptor_sets(&writes, &[]); }
        }


        // --- Collect Prepared Draw Data ---
        for command in render_commands {
            if let Some(resources) = self.entity_cache.get(&command.entity_id) {
                prepared_draws.push(crate::PreparedDrawData {
                    pipeline: resources.pipeline,
                    vertex_buffer: resources.vertex_buffer,
                    vertex_count: resources.vertex_count,
                    descriptor_set: resources.descriptor_set,
                });
            } else {
                // This shouldn't happen if the logic above is correct
                error!("[BufferManager] Resources not found for Entity {:?} during draw data collection!", command.entity_id);
            }
        }

        // TODO: Add logic here to detect entities that were *not* in render_commands
        // but *are* in the cache, and remove their resources. This requires tracking
        // entity lifetimes or using Bevy's RemovedComponents. Deferring for now.

        prepared_draws
    }

    /// Cleans up all managed Vulkan resources.
    pub fn cleanup(
        &mut self,
        platform: &mut VulkanContext,
        // descriptor_pool parameter removed (owned by self now)
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

            // Free descriptor sets explicitly before destroying the pool
            // (Required if pool wasn't created with FREE_DESCRIPTOR_SET_BIT, good practice anyway)
            if !sets_to_free.is_empty() {
                 // Use the pool stored in self
                 match device.free_descriptor_sets(self.descriptor_pool, &sets_to_free) {
                    Ok(_) => info!("[BufferManager::cleanup] Freed {} cached descriptor sets", sets_to_free.len()),
                    Err(e) => error!("[BufferManager::cleanup] Failed to free descriptor sets: {:?}", e),
                 }
            } else {
                 info!("[BufferManager::cleanup] No cached descriptor sets to free.");
            }

            for (_entity_id, mut resources) in self.entity_cache.drain() {
                device.destroy_pipeline(resources.pipeline, None);
                device.destroy_shader_module(resources.vertex_shader, None);
                device.destroy_shader_module(resources.fragment_shader, None);
                // Allocations are now stored directly, not Option
                allocator.destroy_buffer(resources.vertex_buffer, &mut resources.vertex_allocation);
                allocator.destroy_buffer(resources.offset_uniform, &mut resources.offset_allocation);
                // Cleanup instancing buffers later if added
            }
            // self.entity_cache is now empty after drain

            // Cleanup uniform buffer (Still owned by BufferManager)
            allocator.destroy_buffer(self.uniform_buffer, &mut self.uniform_allocation);
            info!("[BufferManager::cleanup] Uniform buffer destroyed");
            self.uniform_buffer = vk::Buffer::null(); // Mark as destroyed
        }
        info!("[BufferManager::cleanup] Finished");
    }
}