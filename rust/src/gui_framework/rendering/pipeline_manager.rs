use ash::vk;
use std::marker::PhantomData;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::scene::scene::Scene;

pub struct PipelineManager {
    pub pipeline_layout: vk::PipelineLayout,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_set: vk::DescriptorSet,
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
                    p_immutable_samplers: std::ptr::null(),
                    _marker: PhantomData,
                },
                vk::DescriptorSetLayoutBinding {
                    binding: 1,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    stage_flags: vk::ShaderStageFlags::VERTEX,
                    p_immutable_samplers: std::ptr::null(),
                    _marker: PhantomData,
                },
            ];
            match device.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo {
                s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::DescriptorSetLayoutCreateFlags::empty(),
                binding_count: bindings.len() as u32,
                p_bindings: bindings.as_ptr(),
                _marker: PhantomData,
            }, None) {
                Ok(layout) => layout,
                Err(e) => panic!("Failed to create descriptor set layout: {:?}", e),
            }
        };

        // Descriptor pool
        let descriptor_pool = unsafe {
            let pool_sizes = [
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 2 * (1 + scene.pool.len() as u32),
                },
            ];
            match device.create_descriptor_pool(&vk::DescriptorPoolCreateInfo {
                s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::DescriptorPoolCreateFlags::empty(),
                max_sets: 1 + scene.pool.len() as u32,
                pool_size_count: pool_sizes.len() as u32,
                p_pool_sizes: pool_sizes.as_ptr(),
                _marker: PhantomData,
            }, None) {
                Ok(pool) => pool,
                Err(e) => panic!("Failed to create descriptor pool: {:?}", e),
            }
        };

        // Allocate descriptor sets (projection + renderables)
        let descriptor_sets = unsafe {
            let layouts = vec![descriptor_set_layout; 1 + scene.pool.len()];
            match device.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo {
                s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
                p_next: std::ptr::null(),
                descriptor_pool,
                descriptor_set_count: layouts.len() as u32,
                p_set_layouts: layouts.as_ptr(),
                _marker: PhantomData,
            }) {
                Ok(sets) => sets,
                Err(e) => panic!("Failed to allocate descriptor sets: {:?}", e),
            }
        };
        let descriptor_set = descriptor_sets[0];

        // Pipeline layout
        let pipeline_layout = unsafe {
            match device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo {
                s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::PipelineLayoutCreateFlags::empty(),
                set_layout_count: 1,
                p_set_layouts: &descriptor_set_layout,
                push_constant_range_count: 0,
                p_push_constant_ranges: std::ptr::null(),
                _marker: PhantomData,
            }, None) {
                Ok(layout) => layout,
                Err(e) => panic!("Failed to create pipeline layout: {:?}", e),
            }
        };

        Self {
            pipeline_layout,
            descriptor_set_layout,
            descriptor_pool,
            descriptor_set,
        }
    }

    pub fn cleanup(self, platform: &mut VulkanContext) {
        let device = platform.device.as_ref().unwrap();
        unsafe {
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
        }
    }
}