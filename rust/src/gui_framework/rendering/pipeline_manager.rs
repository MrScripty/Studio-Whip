use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::scene::scene::Scene;

pub struct PipelineManager {
    pub pipeline_layout: vk::PipelineLayout,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_set: vk::DescriptorSet, // Global projection set
}

impl PipelineManager {
    pub fn new(platform: &mut VulkanContext, scene: &Scene) -> Self {
        let device = platform.device.as_ref().unwrap();

        // Descriptor set layout
        let descriptor_set_layout = unsafe {
            let bindings = [
                vk::DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    stage_flags: vk::ShaderStageFlags::VERTEX,
                    ..Default::default()
                },
                vk::DescriptorSetLayoutBinding {
                    binding: 1,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    stage_flags: vk::ShaderStageFlags::VERTEX,
                    ..Default::default()
                },
            ];
            match device.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo {
                s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
                binding_count: bindings.len() as u32,
                p_bindings: bindings.as_ptr(),
                ..Default::default()
            }, None) {
                Ok(layout) => layout,
                Err(e) => panic!("Failed to create descriptor set layout: {:?}", e),
            }
        };

        // Descriptor pool (Increased sizes)
        let descriptor_pool = unsafe {
            let num_renderables = scene.pool.len() as u32;
            let max_total_sets = 1 + num_renderables;
            let ubo_descriptors_needed = 1 + (2 * num_renderables);

            let pool_sizes = [
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: ubo_descriptors_needed * 2, // Generous count
                },
            ];
            match device.create_descriptor_pool(&vk::DescriptorPoolCreateInfo {
                s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
                flags: vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET,
                max_sets: max_total_sets * 2, // Generous count
                pool_size_count: pool_sizes.len() as u32,
                p_pool_sizes: pool_sizes.as_ptr(),
                ..Default::default()
            }, None) {
                Ok(pool) => pool,
                Err(e) => panic!("Failed to create descriptor pool: {:?}", e),
            }
        };

        // Allocate the *global* descriptor set
        // --- CORRECTED ALLOCATION ---
        let descriptor_set = unsafe {
             match device.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo {
                 s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
                 descriptor_pool,
                 descriptor_set_count: 1,
                 p_set_layouts: &descriptor_set_layout,
                 ..Default::default()
             }) {
                 Ok(sets) => sets[0], // Assign the first (and only) allocated set
                 Err(e) => panic!("Failed to allocate global descriptor set: {:?}", e),
             }
        };
        // --- END CORRECTION ---


        // Pipeline layout
        let pipeline_layout = unsafe {
            match device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo {
                s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
                set_layout_count: 1,
                p_set_layouts: &descriptor_set_layout, // Use the single layout
                ..Default::default()
            }, None) {
                Ok(layout) => layout,
                Err(e) => panic!("Failed to create pipeline layout: {:?}", e),
            }
        };

        Self {
            pipeline_layout,
            descriptor_set_layout,
            descriptor_pool,
            descriptor_set, // Store the allocated global set
        }
    }

    // cleanup remains the same
    pub fn cleanup(self, device: &ash::Device) {
        unsafe {
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            // Freeing individual sets happens in BufferManager::cleanup
            device.destroy_descriptor_pool(self.descriptor_pool, None); // Destroy pool after sets are freed
        }
    }
}