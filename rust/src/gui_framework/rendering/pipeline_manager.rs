use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use bevy_log::info;

pub struct PipelineManager {
    // Layout for per-entity data (Set 0: Global UBO, Transform UBO) - Used by shapes & text
    pub per_entity_layout: vk::DescriptorSetLayout,
    // Layout for text atlas sampler (Set 1) - Used only by text
    pub atlas_layout: vk::DescriptorSetLayout,

    // Pipeline layout using only per-entity data (for shapes)
    pub shape_pipeline_layout: vk::PipelineLayout,
    // Pipeline layout using per-entity data (Set 0) AND atlas sampler (Set 1) (for text)
    pub text_pipeline_layout: vk::PipelineLayout,

    // --- Shared ---
    pub descriptor_pool: vk::DescriptorPool, // Shared pool
}

impl PipelineManager {
    pub fn new(platform: &mut VulkanContext) -> Self {
        let device = platform.device.as_ref().unwrap();

        // --- Per-Entity Descriptor Set Layout (Set 0: Global UBO, Transform UBO) ---
        let per_entity_layout = unsafe {
            let bindings = [
                // Binding 0: Global Projection UBO
                vk::DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    stage_flags: vk::ShaderStageFlags::VERTEX,
                    ..Default::default()
                },
                // Binding 1: Per-Object Transform UBO
                vk::DescriptorSetLayoutBinding {
                    binding: 1,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    stage_flags: vk::ShaderStageFlags::VERTEX,
                    ..Default::default()
                },
            ];
            device.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo {
                s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
                binding_count: bindings.len() as u32,
                p_bindings: bindings.as_ptr(),
                ..Default::default()
            }, None).expect("Failed to create per-entity descriptor set layout")
        };

        // --- Atlas Sampler Descriptor Set Layout (Set 1) ---
        let atlas_layout = unsafe {
            let bindings = [
               // Binding 0: Glyph Atlas Sampler
               vk::DescriptorSetLayoutBinding {
                   binding: 0,
                   descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                   descriptor_count: 1,
                   stage_flags: vk::ShaderStageFlags::FRAGMENT,
                   ..Default::default()
               },
           ];
            device.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo {
               s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
               binding_count: bindings.len() as u32,
               p_bindings: bindings.as_ptr(),
               ..Default::default()
           }, None).expect("Failed to create atlas sampler descriptor set layout")
       };

        // --- Shared Descriptor Pool (Increased size estimate) ---
        let descriptor_pool = unsafe {
            let max_shapes_estimate = 1000u32;
            let max_text_entities_estimate = 100u32; // Estimate for text entities needing atlas sampler sets
            // Max sets: Shape sets (1 UBO per shape) + Text sets (1 sampler per text entity)
            let max_total_sets = max_shapes_estimate + max_text_entities_estimate;
            // Descriptors needed: UBOs for shapes + Samplers for text
            let ubo_descriptors_needed = 1 + max_shapes_estimate; // 1 global proj + 1 per-shape offset
            let sampler_descriptors_needed = max_text_entities_estimate; // 1 per text entity
            let pool_sizes = [
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: ubo_descriptors_needed,
                },
                 vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: sampler_descriptors_needed,
                },
            ];
            device.create_descriptor_pool(&vk::DescriptorPoolCreateInfo {
                s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
                flags: vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET,
                max_sets: max_total_sets,
                pool_size_count: pool_sizes.len() as u32,
                p_pool_sizes: pool_sizes.as_ptr(),
                ..Default::default()
            }, None).expect("Failed to create descriptor pool") // Use expect directly
        };

        // --- Shape Pipeline Layout (Uses Set 0: per_entity_layout) ---
        let shape_pipeline_layout = unsafe {
            device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo {
                s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
                set_layout_count: 1,
                p_set_layouts: &per_entity_layout, // Use the per-entity layout
                ..Default::default()
            }, None).expect("Failed to create shape pipeline layout")
        };

        // --- Text Pipeline Layout (Uses Set 0: per_entity_layout AND Set 1: atlas_layout) ---
        let text_pipeline_layout = unsafe {
            let set_layouts = [per_entity_layout, atlas_layout]; // Use both layouts
            device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo {
                s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
                set_layout_count: set_layouts.len() as u32,
                p_set_layouts: set_layouts.as_ptr(),
                ..Default::default()
            }, None).expect("Failed to create text pipeline layout")
        };

        Self {
            per_entity_layout,
            atlas_layout,
            shape_pipeline_layout,
            text_pipeline_layout,
            descriptor_pool,
        }
    }

}