use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use bevy_log::info;
use std::mem; // Import mem for size_of

/// Helper struct created during initialization to manage the creation of
/// pipeline layouts, descriptor set layouts, and a shared descriptor pool.
/// These resources are then transferred to VulkanContext or Renderer.
pub struct PipelineManager {
    // Descriptor Set Layouts
    pub per_entity_layout: vk::DescriptorSetLayout, // Set 0 (Global UBO, Transform UBO)
    pub atlas_layout: vk::DescriptorSetLayout,      // Set 1 (Atlas Sampler)

    // Pipeline Layouts
    pub shape_pipeline_layout: vk::PipelineLayout, // Uses Set 0 + Push Constants
    pub text_pipeline_layout: vk::PipelineLayout,  // Uses Set 0 + Set 1

    // Shared Pool
    pub descriptor_pool: vk::DescriptorPool,
}

impl PipelineManager {
    pub fn new(platform: &mut VulkanContext) -> Self {
        info!("Creating PipelineManager...");
        let device = platform.device.as_ref().expect("Device missing in PipelineManager::new");

        // --- 1. Create Descriptor Set Layouts ---

        // Layout for Set 0 (Per-Entity Data: Global Projection UBO, Transform UBO)
        let per_entity_bindings = [
            // Binding 0: Global Projection Matrix (Vertex Shader)
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            // Binding 1: Object Transform Matrix (Vertex Shader)
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
        ];
        let per_entity_layout_info = vk::DescriptorSetLayoutCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            binding_count: per_entity_bindings.len() as u32,
            p_bindings: per_entity_bindings.as_ptr(),
            ..Default::default()
        };
        let per_entity_layout = unsafe {
            device.create_descriptor_set_layout(&per_entity_layout_info, None)
        }.expect("Failed to create per-entity descriptor set layout (Set 0)");
        info!("Per-entity descriptor set layout (Set 0) created.");

        // Layout for Set 1 (Global Glyph Atlas Sampler)
        let atlas_bindings = [
            // Binding 0: Glyph Atlas Sampler (Fragment Shader)
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ];
        let atlas_layout_info = vk::DescriptorSetLayoutCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            binding_count: atlas_bindings.len() as u32,
            p_bindings: atlas_bindings.as_ptr(),
            ..Default::default()
        };
        let atlas_layout = unsafe {
            device.create_descriptor_set_layout(&atlas_layout_info, None)
        }.expect("Failed to create atlas descriptor set layout (Set 1)");
        info!("Atlas descriptor set layout (Set 1) created.");

        // --- 2. Create Pipeline Layouts ---

        // Shape Pipeline Layout (Uses Set 0 + Push Constants)
        let shape_set_layouts = [per_entity_layout];
        // Define the push constant range for color (vec4) in the fragment shader
        let push_constant_ranges = [
            vk::PushConstantRange {
                stage_flags: vk::ShaderStageFlags::FRAGMENT, // Color used in fragment shader
                offset: 0,
                size: mem::size_of::<[f32; 4]>() as u32, // Size of a vec4 (RGBA)
            }
        ];
        let shape_pipeline_layout_info = vk::PipelineLayoutCreateInfo {
            s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
            set_layout_count: shape_set_layouts.len() as u32,
            p_set_layouts: shape_set_layouts.as_ptr(),
            push_constant_range_count: push_constant_ranges.len() as u32, // Use push constants
            p_push_constant_ranges: push_constant_ranges.as_ptr(),      // Point to the range
            ..Default::default()
        };
        let shape_pipeline_layout = unsafe {
            device.create_pipeline_layout(&shape_pipeline_layout_info, None)
        }.expect("Failed to create shape pipeline layout");
        info!("Shape pipeline layout created (with push constants).");

        // Text Pipeline Layout (Uses Set 0 + Set 1)
        let text_set_layouts = [per_entity_layout, atlas_layout];
        let text_pipeline_layout_info = vk::PipelineLayoutCreateInfo {
            s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
            set_layout_count: text_set_layouts.len() as u32,
            p_set_layouts: text_set_layouts.as_ptr(),
            push_constant_range_count: 0, // No push constants for text pipeline (yet)
            p_push_constant_ranges: std::ptr::null(),
            ..Default::default()
        };
        let text_pipeline_layout = unsafe {
            device.create_pipeline_layout(&text_pipeline_layout_info, None)
        }.expect("Failed to create text pipeline layout");
        info!("Text pipeline layout created.");

        // --- 3. Create Shared Descriptor Pool ---
        // Estimate pool sizes (adjust as needed)
        let pool_sizes = [
            // For Global UBO + Transform UBOs (Set 0) - Assume max ~1000 entities
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1000 * 2, // 1 global + 1 per entity transform
            },
            // For Atlas Sampler (Set 1) - Only 1 needed globally
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
            },
            // Add other types if needed later
        ];
        let descriptor_pool_info = vk::DescriptorPoolCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
            // Allow freeing individual sets (needed for per-entity cleanup)
            flags: vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET,
            max_sets: 1001, // Max sets = 1 global + 1000 entity sets + 1 atlas set
            pool_size_count: pool_sizes.len() as u32,
            p_pool_sizes: pool_sizes.as_ptr(),
            ..Default::default()
        };
        let descriptor_pool = unsafe {
            device.create_descriptor_pool(&descriptor_pool_info, None)
        }.expect("Failed to create shared descriptor pool");
        info!("Shared descriptor pool created.");

        Self {
            per_entity_layout,
            atlas_layout,
            shape_pipeline_layout,
            text_pipeline_layout,
            descriptor_pool,
        }
    }
}