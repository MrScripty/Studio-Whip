use bevy_log::warn;
use ash::vk;
use vk_mem::Alloc;
use crate::gui_framework::context::vulkan_context::VulkanContext;
// Removed: use crate::gui_framework::scene::scene::Scene;
use crate::gui_framework::rendering::renderable::Renderable;
use bevy_math::Mat4;

pub struct BufferManager {
    pub uniform_buffer: vk::Buffer,
    pub uniform_allocation: vk_mem::Allocation,
    pub renderables: Vec<Renderable>, // This will likely be removed/changed in Step 7
    // Removed descriptor_set_layout and descriptor_pool fields
}

impl BufferManager {
    // Modified signature: Removed scene parameter
    // NOTE: This function is now mostly a placeholder. Buffer creation
    // needs to be driven by ECS components (ShapeData) later in Step 7.
    pub fn new(
        platform: &mut VulkanContext,
        // scene: &Scene, // Removed
        _pipeline_layout: vk::PipelineLayout, // Mark unused for now
        _descriptor_set_layout: vk::DescriptorSetLayout, // Mark unused for now
        _descriptor_pool: vk::DescriptorPool, // Mark unused for now
    ) -> Self {
        println!("[BufferManager::new] Called (ECS Migration - Scene param removed, needs rework)");
        let _device = platform.device.as_ref().unwrap(); // Mark unused for now
        let allocator = platform.allocator.as_ref().unwrap();

        // --- Uniform Buffer Setup (Keep this part) ---
        let ortho = Mat4::orthographic_rh(0.0, 600.0, 300.0, 0.0, -1.0, 1.0); // Use default size initially
        let (uniform_buffer, uniform_allocation) = { // Block starts here
            // Restore BufferCreateInfo initializer
            let buffer_info = vk::BufferCreateInfo {
                s_type: vk::StructureType::BUFFER_CREATE_INFO,
                size: std::mem::size_of::<Mat4>() as u64,
                usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            };
            // Restore AllocationCreateInfo initializer
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
                        (buffer, allocation) // This tuple is the value of the block
                    }
                    Err(e) => panic!("Uniform buffer creation failed: {:?}", e),
                }
            }
        };
        println!("[BufferManager::new] Initial uniform buffer created");

        // --- Removed Processing Each Object in Scene ---
        // The logic to create vertex buffers, offset buffers, instance buffers,
        // descriptor sets, shaders, and pipelines per object is REMOVED here.
        // This needs to happen dynamically based on ECS queries in Step 7.
        let renderables = Vec::new(); // Start with an empty list

        warn!("[BufferManager::new] Object buffer/pipeline creation skipped (Needs ECS implementation in Step 7)");

        Self {
            uniform_buffer,
            uniform_allocation,
            renderables, // Initially empty
        }
    }

    // --- Update functions need complete rework for ECS ---
    // These relied on the old `renderables` Vec structure and indices.
    pub fn update_offset(renderables: &mut Vec<Renderable>, _device: &ash::Device, allocator: &vk_mem::Allocator, index: usize, offset: [f32; 2]) {
         warn!("[BufferManager::update_offset] Called but needs ECS rework.");
         // Old logic commented out:
         /*
         if index >= renderables.len() { return; }
         let renderable = &mut renderables[index];
         unsafe { ... }
         */
    }

    pub fn update_instance_offset(renderables: &mut Vec<Renderable>, _device: &ash::Device, allocator: &vk_mem::Allocator, object_index: usize, instance_id: usize, offset: [f32; 2]) {
        warn!("[BufferManager::update_instance_offset] Called but needs ECS rework.");
        // Old logic commented out:
        /*
        if object_index >= renderables.len() { return; }
        let renderable = &mut renderables[object_index];
        if let Some(ref mut instance_allocation) = renderable.instance_allocation { ... }
        */
    }

    pub fn update_instance_buffer(
        renderables: &mut Vec<Renderable>,
        _device: &ash::Device,
        allocator: &vk_mem::Allocator,
        object_id: usize,
        instance_id: usize,
        offset: [f32; 2],
    ) {
        warn!("[BufferManager::update_instance_buffer] Called but needs ECS rework.");
        // Old logic commented out:
        /*
        if object_id >= renderables.len() { ... }
        let renderable = &mut renderables[object_id];
        if renderable.instance_buffer.is_none() || renderable.instance_allocation.is_none() { ... }
        if instance_id != renderable.instance_count as usize { ... }
        if renderable.instance_count >= renderable.instance_buffer_capacity { ... }
        if let Some(ref mut instance_allocation) = renderable.instance_allocation { ... }
        */
    }

    // Cleanup needs rework based on how buffers are managed with ECS
    pub fn cleanup(
        mut self,
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
        descriptor_pool: vk::DescriptorPool
    ) {
        println!("[BufferManager::cleanup] Called (Needs ECS rework)");
        unsafe {
            // Free descriptor sets (if any were created - currently none)
            let sets_to_free: Vec<vk::DescriptorSet> = self.renderables.iter().map(|r| r.descriptor_set).collect();
            if !sets_to_free.is_empty() {
                 device.free_descriptor_sets(descriptor_pool, &sets_to_free).unwrap();
                 println!("[BufferManager::cleanup] Freed {} descriptor sets", sets_to_free.len());
            }

            // Cleanup renderables (if any exist - currently none created in new())
            println!("[BufferManager::cleanup] Cleaning up {} renderables...", self.renderables.len());
            for mut renderable in self.renderables.drain(..) { // Use drain
                device.destroy_pipeline(renderable.pipeline, None);
                device.destroy_shader_module(renderable.vertex_shader, None);
                device.destroy_shader_module(renderable.fragment_shader, None);
                allocator.destroy_buffer(renderable.vertex_buffer, &mut renderable.vertex_allocation);
                allocator.destroy_buffer(renderable.offset_uniform, &mut renderable.offset_allocation);
                if let (Some(instance_buffer), Some(mut instance_allocation)) = (renderable.instance_buffer.take(), renderable.instance_allocation.take()) {
                    allocator.destroy_buffer(instance_buffer, &mut instance_allocation);
                }
            }

            // Cleanup uniform buffer (Keep this)
            allocator.destroy_buffer(self.uniform_buffer, &mut self.uniform_allocation);
            println!("[BufferManager::cleanup] Uniform buffer destroyed");
        }
        println!("[BufferManager::cleanup] Finished");
    }
}