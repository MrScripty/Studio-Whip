// In src/gui_framework/rendering/buffer_manager.rs

use bevy_log::{info, error, warn}; // Added warn
use ash::vk;
use vk_mem::Alloc;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use bevy_math::Mat4;
use std::collections::HashMap;
use bevy_ecs::prelude::Entity;
use crate::Vertex;

// --- Struct definitions (EntityRenderResources, PipelineCacheKey, ShaderCacheKey) remain the same ---
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
    pub uniform_buffer: vk::Buffer,
    pub uniform_allocation: vk_mem::Allocation,
    entity_cache: HashMap<Entity, EntityRenderResources>,
    pipeline_cache: HashMap<PipelineCacheKey, vk::Pipeline>,
    shader_cache: HashMap<ShaderCacheKey, vk::ShaderModule>,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
}

impl BufferManager {
    // --- new() function remains the same ---
    pub fn new(
        platform: &mut VulkanContext,
        _pipeline_layout: vk::PipelineLayout,
        descriptor_set_layout: vk::DescriptorSetLayout,
        descriptor_pool: vk::DescriptorPool,
    ) -> Self {
        info!("[BufferManager::new] Called (ECS Migration - Reworking)");
        let allocator = platform.allocator.as_ref().unwrap();
        // --- Uniform Buffer Setup ---
        // (Projection matrix calculation removed - now done in Renderer::new)
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
            // Initial write is now done in Renderer::new after this BufferManager is created
            unsafe {
                allocator.create_buffer(&buffer_info, &allocation_info)
                    .expect("Uniform buffer creation failed")
            }
        };
        info!("[BufferManager::new] Initial uniform buffer created (data written in Renderer::new)");

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
            descriptor_set_layout,
            descriptor_pool,
        }
    }


    // --- UPDATED prepare_frame_resources ---
    pub fn prepare_frame_resources(
        &mut self,
        platform: &mut VulkanContext,
        render_commands: &[crate::RenderCommandData], // Includes vertices_changed flag
    ) -> Vec<crate::PreparedDrawData> {
        let device = platform.device.as_ref().expect("Device missing in prepare_frame_resources");
        let allocator = platform.allocator.as_ref().expect("Allocator missing in prepare_frame_resources");
        let render_pass = platform.render_pass.expect("Render pass missing in prepare_frame_resources");
        let pipeline_layout = platform.pipeline_layout.expect("Pipeline layout missing in prepare_frame_resources");

        let mut prepared_draws = Vec::with_capacity(render_commands.len());
        // `created_entities` vector removed - no longer needed

        for command in render_commands {
            let entity_id = command.entity_id;

            // --- Handle Existing or New Entity ---
            if let Some(resources) = self.entity_cache.get_mut(&entity_id) {
                // --- Existing Entity ---

                // Check if vertex data changed
                if command.vertices_changed {
                    info!("[BufferManager] Vertices changed for {:?}. Updating vertex buffer.", entity_id);
                    let new_vertex_count = command.vertices.len();
                    let new_size_bytes = (std::mem::size_of::<Vertex>() * new_vertex_count) as u64;

                    // Get current allocation info
                    let current_alloc_info = allocator.get_allocation_info(&resources.vertex_allocation);

                    // Recreate buffer ONLY if size changed
                    if new_size_bytes != current_alloc_info.size {
                        warn!("[BufferManager] Vertex buffer size changed for {:?} ({} bytes -> {} bytes). Recreating buffer.",
                              entity_id, current_alloc_info.size, new_size_bytes);
                        // Cleanup old vertex buffer/allocation
                        unsafe { allocator.destroy_buffer(resources.vertex_buffer, &mut resources.vertex_allocation); }

                        // Create new vertex buffer
                        let (new_buffer, new_alloc) = unsafe {
                            let buffer_info = vk::BufferCreateInfo {
                                s_type: vk::StructureType::BUFFER_CREATE_INFO,
                                size: new_size_bytes, // Use new size
                                usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                                sharing_mode: vk::SharingMode::EXCLUSIVE,
                                ..Default::default()
                            };
                            let allocation_info = vk_mem::AllocationCreateInfo {
                                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                                flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE | vk_mem::AllocationCreateFlags::MAPPED,
                                ..Default::default()
                            };
                            allocator.create_buffer(&buffer_info, &allocation_info)
                                     .expect("Failed to recreate vertex buffer")
                        };
                        // Update the cached resources
                        resources.vertex_buffer = new_buffer;
                        resources.vertex_allocation = new_alloc;
                    }

                    // Copy new vertex data into the (potentially new) buffer's mapped memory
                    unsafe {
                        // Need to get allocation info again in case it was recreated
                        let info = allocator.get_allocation_info(&resources.vertex_allocation);
                        if !info.mapped_data.is_null() {
                            let data_ptr = info.mapped_data.cast::<Vertex>();
                            data_ptr.copy_from_nonoverlapping(command.vertices.as_ptr(), new_vertex_count);
                            // No need to unmap persistently mapped memory
                        } else {
                             error!("[BufferManager] Vertex buffer allocation not mapped during update for {:?}!", entity_id);
                        }
                    }
                    // Update the vertex count in the cache
                    resources.vertex_count = new_vertex_count as u32;
                } // End if vertices_changed

                // --- Update Offset UBO (Always for existing entities) ---
                // (Commented out excessive logging)
                // bevy_log::info!("Updating UBO for Entity {:?}, Matrix:\n{:?}", entity_id, command.transform_matrix);
                unsafe {
                    let info = allocator.get_allocation_info(&resources.offset_allocation);
                    if !info.mapped_data.is_null() {
                        let data_ptr = info.mapped_data.cast::<f32>();
                        data_ptr.copy_from_nonoverlapping(command.transform_matrix.to_cols_array().as_ptr(), 16);
                    } else {
                         error!("[BufferManager] Offset UBO allocation not mapped during update for {:?}!", entity_id);
                    }
                }

            } else {
                // --- New Entity ---
                info!("[BufferManager] Creating resources for Entity {:?}", entity_id);
                let vertex_count = command.vertices.len(); // Get count from command
                let vertex_buffer_size = (std::mem::size_of::<Vertex>() * vertex_count) as u64;

                // 1. Create Vertex Buffer
                let (vertex_buffer, vertex_allocation) = unsafe {
                    let buffer_info = vk::BufferCreateInfo {
                        s_type: vk::StructureType::BUFFER_CREATE_INFO,
                        size: vertex_buffer_size, // Use calculated size
                        usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                        sharing_mode: vk::SharingMode::EXCLUSIVE,
                        ..Default::default()
                    };
                    let allocation_info = vk_mem::AllocationCreateInfo {
                        usage: vk_mem::MemoryUsage::AutoPreferDevice,
                        flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE | vk_mem::AllocationCreateFlags::MAPPED,
                        ..Default::default()
                    };
                    allocator.create_buffer(&buffer_info, &allocation_info)
                        .expect("Failed to create vertex buffer")
                };
                // Copy initial vertex data
                unsafe {
                    let info = allocator.get_allocation_info(&vertex_allocation);
                    assert!(!info.mapped_data.is_null(), "Vertex allocation should be mapped but pointer is null");
                    let data_ptr = info.mapped_data.cast::<Vertex>();
                    data_ptr.copy_from_nonoverlapping(command.vertices.as_ptr(), vertex_count); // Use vertex_count
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
                 // Copy initial transform matrix to offset UBO for new entity
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
                };
                // Pipeline creation logic remains the same...
                let _pipeline = *self.pipeline_cache.entry(pipeline_key.clone()).or_insert_with(|| {
                    info!("[BufferManager] Creating and caching pipeline for key: {:?}", pipeline_key);
                    unsafe {
                        // ... (shader_stages, vertex_attr_descs, vertex_binding_descs, etc.) ...
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
                        let vertex_attr_descs = [
                            vk::VertexInputAttributeDescription {
                                location: 0, binding: 0, format: vk::Format::R32G32_SFLOAT, offset: 0,
                            },
                        ];
                        let vertex_binding_descs = [
                            vk::VertexInputBindingDescription {
                                binding: 0, stride: std::mem::size_of::<Vertex>() as u32, input_rate: vk::VertexInputRate::VERTEX,
                            },
                             vk::VertexInputBindingDescription { // Keep instance binding definition even if unused by some pipelines
                                binding: 1, stride: (std::mem::size_of::<f32>() * 4) as u32, input_rate: vk::VertexInputRate::INSTANCE,
                            },
                        ];
                        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
                            vertex_binding_description_count: 1, // Adjust if using instancing later
                            p_vertex_binding_descriptions: vertex_binding_descs.as_ptr(),
                            vertex_attribute_description_count: 1,
                            p_vertex_attribute_descriptions: vertex_attr_descs.as_ptr(),
                            ..Default::default()
                        };
                        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
                            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                            ..Default::default()
                        };
                        let viewport_state = vk::PipelineViewportStateCreateInfo {
                            viewport_count: 1, scissor_count: 1, ..Default::default()
                        };
                        let rasterizer = vk::PipelineRasterizationStateCreateInfo {
                            polygon_mode: vk::PolygonMode::FILL, line_width: 1.0, cull_mode: vk::CullModeFlags::NONE, front_face: vk::FrontFace::CLOCKWISE, ..Default::default()
                        };
                        let multisampling = vk::PipelineMultisampleStateCreateInfo {
                            rasterization_samples: vk::SampleCountFlags::TYPE_1, ..Default::default()
                        };
                        let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
                            color_write_mask: vk::ColorComponentFlags::RGBA, ..Default::default()
                        };
                        let color_blending = vk::PipelineColorBlendStateCreateInfo {
                            attachment_count: 1, p_attachments: &color_blend_attachment, ..Default::default()
                        };
                         let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
                         let dynamic_state_info = vk::PipelineDynamicStateCreateInfo {
                             dynamic_state_count: dynamic_states.len() as u32, p_dynamic_states: dynamic_states.as_ptr(), ..Default::default()
                         };
                        let pipeline_info = vk::GraphicsPipelineCreateInfo {
                            stage_count: shader_stages.len() as u32, p_stages: shader_stages.as_ptr(), p_vertex_input_state: &vertex_input_info, p_input_assembly_state: &input_assembly, p_viewport_state: &viewport_state, p_rasterization_state: &rasterizer, p_multisample_state: &multisampling, p_color_blend_state: &color_blending, p_dynamic_state: &dynamic_state_info, layout: pipeline_layout, render_pass, subpass: 0, ..Default::default()
                        };
                        device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                            .expect("Failed to create graphics pipeline").remove(0)
                    }
                });

                // 6. Store resources in cache
                let resources = EntityRenderResources {
                    vertex_buffer, vertex_allocation,
                    vertex_count: vertex_count as u32, // Store initial count
                    offset_uniform, offset_allocation,
                    descriptor_set,
                };
                self.entity_cache.insert(entity_id, resources);
                // created_entities.push(entity_id); // Removed
            } // End if/else for existing/new entity
        } // End of render_commands loop


        // --- Update Descriptor Sets (Always update for all entities in the frame) ---
        let mut writes = Vec::with_capacity(render_commands.len() * 2);
        let mut proj_buffer_info: Option<vk::DescriptorBufferInfo> = None;

        for command in render_commands {
             if let Some(resources) = self.entity_cache.get(&command.entity_id) {
                 if proj_buffer_info.is_none() {
                     proj_buffer_info = Some(vk::DescriptorBufferInfo {
                         buffer: self.uniform_buffer, offset: 0, range: std::mem::size_of::<Mat4>() as u64,
                     });
                 }
                 let offset_buffer_info = vk::DescriptorBufferInfo {
                     buffer: resources.offset_uniform, offset: 0, range: std::mem::size_of::<Mat4>() as u64,
                 };
                 // Write Binding 0 (Global Projection)
                 writes.push(vk::WriteDescriptorSet {
                     s_type: vk::StructureType::WRITE_DESCRIPTOR_SET, dst_set: resources.descriptor_set, dst_binding: 0, descriptor_count: 1, descriptor_type: vk::DescriptorType::UNIFORM_BUFFER, p_buffer_info: proj_buffer_info.as_ref().unwrap(), ..Default::default()
                 });
                 // Write Binding 1 (Per-Object Offset)
                 writes.push(vk::WriteDescriptorSet {
                     s_type: vk::StructureType::WRITE_DESCRIPTOR_SET, dst_set: resources.descriptor_set, dst_binding: 1, descriptor_count: 1, descriptor_type: vk::DescriptorType::UNIFORM_BUFFER, p_buffer_info: &offset_buffer_info, ..Default::default()
                 });
             }
        }
        if !writes.is_empty() {
             unsafe { device.update_descriptor_sets(&writes, &[]); }
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
                prepared_draws.push(crate::PreparedDrawData {
                    pipeline: *pipeline,
                    vertex_buffer: resources.vertex_buffer,
                    vertex_count: resources.vertex_count, // Use the potentially updated count
                    descriptor_set: resources.descriptor_set,
                });
            } else {
                error!("[BufferManager] Resources not found for Entity {:?} during draw data collection!", command.entity_id);
            }
        }

        // TODO: Implement resource removal for despawned entities using RemovedComponents<ShapeData> query.

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

            // Cleanup uniform buffer
            info!(
                "[BufferManager::cleanup] Destroying Global Uniform Buffer: {:?}, Allocation: {:?}",
                self.uniform_buffer, self.uniform_allocation
            );
            allocator.destroy_buffer(self.uniform_buffer, &mut self.uniform_allocation);
            info!("[BufferManager::cleanup] Uniform buffer destroyed");

        }
        info!("[BufferManager::cleanup] Finished");
    }
}