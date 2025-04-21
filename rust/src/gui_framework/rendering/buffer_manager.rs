use bevy_log::{warn, info};
use ash::vk;
use vk_mem::Alloc; // Import Allocation directly
use crate::gui_framework::context::vulkan_context::VulkanContext;
// Removed Renderable import
// use crate::gui_framework::rendering::renderable::Renderable;
use bevy_math::Mat4;
use std::collections::HashMap; // Needed for caching later
use bevy_ecs::prelude::Entity; // Needed for caching later
use crate::Vertex; // Needed for vertex buffer size

// Placeholder struct for cached per-entity resources
// This will evolve significantly in the next phase
struct EntityRenderResources {
    vertex_buffer: vk::Buffer,
    vertex_allocation: Option<vk_mem::Allocation>, // Use Option
    vertex_count: u32,
    offset_uniform: vk::Buffer,
    offset_allocation: Option<vk_mem::Allocation>, // Use Option
    descriptor_set: vk::DescriptorSet,
    pipeline: vk::Pipeline,
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

    // --- NEW Method: Prepare resources for the frame ---
    pub fn prepare_frame_resources(
        &mut self,
        platform: &mut VulkanContext,
        render_commands: &[crate::RenderCommandData], // Use RenderCommandData from lib.rs
    ) -> Vec<crate::PreparedDrawData> { // Return Vec of PreparedDrawData
        warn!("[BufferManager::prepare_frame_resources] NOT IMPLEMENTED - Needs major rework!");
        // --- Placeholder Implementation ---
        let device = platform.device.as_ref().unwrap();
        let _allocator = platform.allocator.as_ref().unwrap(); // Prefix unused allocator

        for command in render_commands {
            if !self.entity_cache.contains_key(&command.entity_id) {
                // --- Create Resources for New Entity (Placeholder) ---
                warn!("[BufferManager] Creating placeholder resources for Entity {:?}", command.entity_id);

                // 1. Create Vertex Buffer (using command.vertices)
                let _vertex_buffer_size = (std::mem::size_of::<Vertex>() * command.vertices.len()) as u64; //Prefix Unused
                let (vertex_buffer, vertex_allocation) = { /* ... vk_mem create_buffer ... */
                    // Placeholder: Use None for Allocation
                    (vk::Buffer::null(), None)
                };

                // 2. Create Offset Uniform Buffer (size = Mat4)
                let (offset_uniform, offset_allocation) = { /* ... vk_mem create_buffer (mapped) ... */
                    // Placeholder: Use None for Allocation
                    (vk::Buffer::null(), None)
                };

                // 3. Allocate Descriptor Set
                let descriptor_set = unsafe {
                    device.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo {
                        s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
                        descriptor_pool: self.descriptor_pool,
                        descriptor_set_count: 1,
                        p_set_layouts: &self.descriptor_set_layout,
                        ..Default::default()
                    }).unwrap()[0] // Basic error handling
                };

                // 4. Update Descriptor Set (Binding 0: Global Projection, Binding 1: Per-Object Offset)
                let proj_buffer_info = vk::DescriptorBufferInfo { /* ... using self.uniform_buffer ... */
                     buffer: self.uniform_buffer, offset: 0, range: std::mem::size_of::<Mat4>() as u64
                };
                let offset_buffer_info = vk::DescriptorBufferInfo { /* ... using offset_uniform ... */
                     buffer: offset_uniform, offset: 0, range: std::mem::size_of::<Mat4>() as u64
                };
                unsafe {
                    device.update_descriptor_sets(&[
                        vk::WriteDescriptorSet { // Binding 0
                            s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                            dst_set: descriptor_set, // The newly allocated set
                            dst_binding: 0,
                            descriptor_count: 1,
                            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                            p_buffer_info: &proj_buffer_info,
                            ..Default::default()
                        },
                        vk::WriteDescriptorSet { // Binding 1
                            s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                            dst_set: descriptor_set, // The newly allocated set
                            dst_binding: 1,
                            descriptor_count: 1,
                            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                            p_buffer_info: &offset_buffer_info,
                            ..Default::default()
                        },
                    ], &[]);
                }


                // 5. Load Shaders (using command.vertex/fragment_shader_path)
                let vertex_shader = crate::gui_framework::rendering::shader_utils::load_shader(device, &command.vertex_shader_path);
                let fragment_shader = crate::gui_framework::rendering::shader_utils::load_shader(device, &command.fragment_shader_path);

                // 6. Create Pipeline (using shaders, layout, render pass from platform)
                let pipeline = { /* ... vk::create_graphics_pipelines ... */
                    // Placeholder:
                    vk::Pipeline::null()
                };

                // 7. Store in cache
                self.entity_cache.insert(command.entity_id, EntityRenderResources {
                    vertex_buffer, vertex_allocation, // Store Option<Allocation>
                    vertex_count: command.vertices.len() as u32,
                    offset_uniform, offset_allocation, // Store Option<Allocation>
                    descriptor_set, pipeline,
                    vertex_shader, fragment_shader,
                });
            }

            // --- Update Offset UBO for Existing/New Entity ---
            if let Some(_resources) = self.entity_cache.get_mut(&command.entity_id) { // Prefix unused
                // Map the offset buffer and copy command.transform_matrix
                // Check if allocation exists before mapping
                // if let Some(ref mut alloc) = resources.offset_allocation {
                //     unsafe {
                //         let data_ptr = allocator.map_memory(alloc).unwrap().cast::<f32>();
                //         data_ptr.copy_from_nonoverlapping(command.transform_matrix.to_cols_array().as_ptr(), 16);
                //         allocator.unmap_memory(alloc);
                //     }
                // }
            }
        }

        // --- Return Data for Drawing (Placeholder) ---
        Vec::new() // Placeholder
    }


    // --- Removed Old Update Functions ---

    // --- Modified Cleanup Signature ---
    pub fn cleanup(
        &mut self, // Changed to &mut self
        platform: &mut VulkanContext, // Takes &mut VulkanContext now
        descriptor_pool: vk::DescriptorPool // Keep pool for freeing sets
    ) {
        info!("[BufferManager::cleanup] Called (&mut self, ECS Rework)");
        let device = platform.device.as_ref().expect("Device missing in cleanup");
        let allocator = platform.allocator.as_ref().expect("Allocator missing in cleanup");

        unsafe {
            // Free descriptor sets from the cache
            let sets_to_free: Vec<vk::DescriptorSet> = self.entity_cache.values()
                .map(|r| r.descriptor_set)
                .collect();
            if !sets_to_free.is_empty() {
                 device.free_descriptor_sets(descriptor_pool, &sets_to_free).unwrap();
                 info!("[BufferManager::cleanup] Freed {} cached descriptor sets", sets_to_free.len());
            } else {
                 info!("[BufferManager::cleanup] No cached descriptor sets to free.");
            }

            // Cleanup cached resources
            info!("[BufferManager::cleanup] Cleaning up {} cached entity resources...", self.entity_cache.len());
            for (_entity_id, mut resources) in self.entity_cache.drain() { // Use drain (works on &mut self)
                device.destroy_pipeline(resources.pipeline, None);
                device.destroy_shader_module(resources.vertex_shader, None);
                device.destroy_shader_module(resources.fragment_shader, None);

                // Check if allocation exists before destroying buffer/allocation
                if let Some(mut alloc) = resources.vertex_allocation.take() { // Use take()
                    allocator.destroy_buffer(resources.vertex_buffer, &mut alloc);
                }
                if let Some(mut alloc) = resources.offset_allocation.take() { // Use take()
                    allocator.destroy_buffer(resources.offset_uniform, &mut alloc);
                }
                // Cleanup instancing buffers later if added
            }
            // self.entity_cache is now empty after drain

            // Cleanup uniform buffer (Still owned by BufferManager)
            // Need mutable access to self.uniform_allocation
            allocator.destroy_buffer(self.uniform_buffer, &mut self.uniform_allocation);
            info!("[BufferManager::cleanup] Uniform buffer destroyed");
            // Set buffer handle to null after destruction? Or rely on drop?
            // Setting to null might prevent double-free if cleanup called again, but shouldn't happen.
            self.uniform_buffer = vk::Buffer::null();
        }
        info!("[BufferManager::cleanup] Finished");
    }
}