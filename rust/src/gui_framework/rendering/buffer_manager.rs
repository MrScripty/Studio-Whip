use bevy_log::{info, error};
use ash::vk;
use vk_mem::Alloc; // Import Allocation directly
use crate::gui_framework::context::vulkan_context::VulkanContext;
use bevy_math::Mat4;
use std::collections::HashMap; // Needed for caching later
use bevy_ecs::prelude::Entity; // Needed for caching later
use crate::Vertex; // Needed for vertex buffer size

// Holds the Vulkan resources cached per entity that are *unique* per entity.
// Pipeline and shaders are now cached separately.
struct EntityRenderResources {
    vertex_buffer: vk::Buffer,
    vertex_allocation: vk_mem::Allocation, // Store Allocation directly
    vertex_count: u32,
    offset_uniform: vk::Buffer,
    offset_allocation: vk_mem::Allocation,
    descriptor_set: vk::DescriptorSet,     // Per-entity descriptor set
    // Pipeline and shaders removed, will be retrieved from cache
    // Add instancing fields later
}

// Key for pipeline cache. Includes shader paths and potentially other state later (e.g., render pass).
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
struct PipelineCacheKey {
    vertex_shader_path: String,
    fragment_shader_path: String,
    // Add render_pass handle or other relevant state if pipelines vary based on them
}

// Key for shader cache. Just the path for now.
type ShaderCacheKey = String;


pub struct BufferManager {
    pub uniform_buffer: vk::Buffer,
    pub uniform_allocation: vk_mem::Allocation, // Global uniform is always valid
    // Replace renderables Vec with a cache
    entity_cache: HashMap<Entity, EntityRenderResources>,
    // Cache for shared pipelines
    pipeline_cache: HashMap<PipelineCacheKey, vk::Pipeline>,
    // Cache for shared shader modules
    shader_cache: HashMap<ShaderCacheKey, vk::ShaderModule>,
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
        let ortho = Mat4::orthographic_rh(0.0, 600.0, 300.0, 0.0, -100.0, 100.0); // Use default size initially
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
                    Ok((buffer, allocation)) => {
                        // Initial write using get_allocation_info for mapped pointer
                        let info = allocator.get_allocation_info(&allocation);
                        if !info.mapped_data.is_null() {
                            let data_ptr = info.mapped_data.cast::<f32>();
                            data_ptr.copy_from_nonoverlapping(ortho.to_cols_array().as_ptr(), 16);
                        } else {
                            error!("[BufferManager::new] Failed to get mapped pointer for initial uniform buffer write.");
                            // Attempt map/unmap as fallback?
                        }
                        // No unmap needed for persistently mapped
                        (buffer, allocation)
                    }
                    Err(e) => panic!("Uniform buffer creation failed: {:?}", e),
                }
            }
        };
        info!("[BufferManager::new] Initial uniform buffer created");

        // --- Initialize Caches ---
        let entity_cache = HashMap::new();
        let pipeline_cache = HashMap::new();
        let shader_cache = HashMap::new();
        info!("[BufferManager::new] Caches initialized (entity, pipeline, shader)");

        Self {
            uniform_buffer,
            uniform_allocation,
            entity_cache,
            pipeline_cache,
            shader_cache,
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
                let (vertex_buffer, vertex_allocation) = unsafe { // Removed mut
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

                // 4. Load/Get Shaders from Cache
                let vertex_shader_path = &command.vertex_shader_path;
                let vertex_shader = *self.shader_cache.entry(vertex_shader_path.clone()).or_insert_with(|| {
                    info!("[BufferManager] Loading and caching shader: {}", vertex_shader_path);
                    crate::gui_framework::rendering::shader_utils::load_shader(device, vertex_shader_path)
                });

                let fragment_shader_path = &command.fragment_shader_path;
                let fragment_shader = *self.shader_cache.entry(fragment_shader_path.clone()).or_insert_with(|| {
                    info!("[BufferManager] Loading and caching shader: {}", fragment_shader_path);
                    crate::gui_framework::rendering::shader_utils::load_shader(device, fragment_shader_path)
                });

                // 5. Create/Get Pipeline from Cache
                let pipeline_key = PipelineCacheKey {
                    vertex_shader_path: command.vertex_shader_path.clone(),
                    fragment_shader_path: command.fragment_shader_path.clone(),
                    // Add render_pass handle here if needed
                };

                let _pipeline = *self.pipeline_cache.entry(pipeline_key.clone()).or_insert_with(|| { 
                    info!("[BufferManager] Creating and caching pipeline for key: {:?}", pipeline_key);
                    // Pipeline creation logic moved inside closure
                    unsafe {
                        let shader_stages = [ // Use cached shader modules
                            vk::PipelineShaderStageCreateInfo {
                                s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                                stage: vk::ShaderStageFlags::VERTEX,
                                module: vertex_shader, // Use cached module
                                p_name: b"main\0".as_ptr() as *const std::ffi::c_char,
                                ..Default::default()
                            },
                            vk::PipelineShaderStageCreateInfo {
                                s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                                stage: vk::ShaderStageFlags::FRAGMENT,
                                module: fragment_shader, // Use cached module
                                p_name: b"main\0".as_ptr() as *const std::ffi::c_char,
                                ..Default::default()
                            },
                        ];
                        // Define only the vertex attributes we are currently using (Location 0).
                        let vertex_attr_descs = [
                            // Location 0: Vertex Position
                            vk::VertexInputAttributeDescription {
                                location: 0,
                                binding: 0, // Matches the binding description below
                                format: vk::Format::R32G32_SFLOAT, // vec2
                                offset: 0, // Position is first in Vertex struct
                            },
                            // Removed placeholder for Location 1
                        ];

                        // Define only the vertex input binding we are currently using (Binding 0).
                        let vertex_binding_descs = [
                            // Binding 0: Per-vertex data
                            vk::VertexInputBindingDescription {
                                binding: 0,
                                stride: std::mem::size_of::<Vertex>() as u32,
                                input_rate: vk::VertexInputRate::VERTEX,
                            },
                             // Binding 1: Per-instance data (stride depends on actual instance data struct)
                             vk::VertexInputBindingDescription {
                                binding: 1,
                                stride: (std::mem::size_of::<f32>() * 4) as u32, // Placeholder stride (vec4)
                                input_rate: vk::VertexInputRate::INSTANCE, // Mark as per-instance
                            },
                        ];


                        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
                            s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
                            // Use the arrays defined above (now only containing binding/location 0)
                            vertex_binding_description_count: 1, // Only binding 0
                            p_vertex_binding_descriptions: vertex_binding_descs.as_ptr(),
                            vertex_attribute_description_count: 1, // Only location 0
                            p_vertex_attribute_descriptions: vertex_attr_descs.as_ptr(),
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
                    } // End unsafe block
                }); // End or_insert_with closure

                // 6. Store *entity-specific* resources in cache
                let resources = EntityRenderResources {
                    vertex_buffer, vertex_allocation,
                    vertex_count: command.vertices.len() as u32,
                    offset_uniform, offset_allocation,
                    descriptor_set,
                    // Pipeline and shaders are now stored in their respective caches
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
        // This is safe now because Renderer::render waits for the fence before calling this.
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


        // --- Collect Prepared Draw Data (Retrieve pipeline from cache) ---
        for command in render_commands {
            // Construct pipeline key again to retrieve from cache
            let pipeline_key = PipelineCacheKey {
                vertex_shader_path: command.vertex_shader_path.clone(),
                fragment_shader_path: command.fragment_shader_path.clone(),
                // Add render_pass handle here if needed
            };
            // Retrieve the pipeline from the cache. It *must* exist at this point.
            let pipeline = self.pipeline_cache.get(&pipeline_key)
                .expect("Pipeline should be in cache but wasn't found!");

            if let Some(resources) = self.entity_cache.get(&command.entity_id) {
                prepared_draws.push(crate::PreparedDrawData {
                    pipeline: *pipeline, // Use the cached pipeline
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

           // Cleanup entity-specific resources
           let entity_count = self.entity_cache.len(); // Get count before draining
           for (entity_id, mut resources) in self.entity_cache.drain() {
               // Log before destroying vertex buffer/allocation
               info!(
                   "[BufferManager::cleanup] Destroying Entity {:?} Vertex Buffer: {:?}, Allocation: {:?}",
                   entity_id, resources.vertex_buffer, resources.vertex_allocation // Allocation debug might be limited
               );
               allocator.destroy_buffer(resources.vertex_buffer, &mut resources.vertex_allocation);

               // Log before destroying offset uniform buffer/allocation
               info!(
                   "[BufferManager::cleanup] Destroying Entity {:?} Offset Uniform: {:?}, Allocation: {:?}",
                   entity_id, resources.offset_uniform, resources.offset_allocation
               );
               allocator.destroy_buffer(resources.offset_uniform, &mut resources.offset_allocation);
               // Cleanup instancing buffers later if added
           }
           info!("[BufferManager::cleanup] Cleaned up {} entity-specific resources.", entity_count); // Log count based on initial size

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

           // self.entity_cache, pipeline_cache, shader_cache are now empty after drain

            // Cleanup uniform buffer (Still owned by BufferManager)
            // Need mutable access to the allocation field itself for destroy_buffer
            info!(
                "[BufferManager::cleanup] Destroying Global Uniform Buffer: {:?}, Allocation: {:?}",
                self.uniform_buffer, self.uniform_allocation
            );
            allocator.destroy_buffer(self.uniform_buffer, &mut self.uniform_allocation);
            info!("[BufferManager::cleanup] Uniform buffer destroyed"); // Keep this confirmation

        }
        info!("[BufferManager::cleanup] Finished");
    }
}