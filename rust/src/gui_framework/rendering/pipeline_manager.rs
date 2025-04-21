use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use bevy_log::info; // Use info! macro

// Represents the PipelineManager struct
pub struct PipelineManager {
    pub pipeline_layout: vk::PipelineLayout,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
}

impl PipelineManager {
    pub fn new(platform: &mut VulkanContext) -> Self {
        info!("[PipelineManager::new] Called (ECS Migration)"); // Use info!
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

        // Descriptor pool (Using estimate)
        let descriptor_pool = unsafe {
            let max_renderables_estimate = 1000u32;
            let max_total_sets = 1 + max_renderables_estimate; // 1 global + estimate per object
            let ubo_descriptors_needed = 1 + (1 * max_renderables_estimate); // 1 global proj + 1 per-object offset
            let pool_sizes = [
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: ubo_descriptors_needed,
                },
                // Add other types here if needed (e.g., combined image sampler)
            ];
            match device.create_descriptor_pool(&vk::DescriptorPoolCreateInfo {
                s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
                flags: vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET,
                max_sets: max_total_sets,
                pool_size_count: pool_sizes.len() as u32,
                p_pool_sizes: pool_sizes.as_ptr(),
                ..Default::default()
            }, None) {
                Ok(pool) => pool,
                Err(e) => panic!("Failed to create descriptor pool: {:?}", e),
            }
        };
        info!("[PipelineManager::new] Descriptor set layout and pool created. Per-entity sets will be allocated by BufferManager.");

        // Pipeline layout
        let pipeline_layout = unsafe {
            match device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo {
                s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
                set_layout_count: 1,
                p_set_layouts: &descriptor_set_layout,
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
        }
    }

}